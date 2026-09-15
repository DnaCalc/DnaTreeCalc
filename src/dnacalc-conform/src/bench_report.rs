//! `bench.v1.json` — the retained artifact of one `bench` run (dtc-5zet.13):
//! the program's first grid-engine wall-clock baseline, measured through
//! the path the product runs (`dnacalc_host_core::WorkbookSession`).
//!
//! Every number here is either a wall-clock sample set (`Samples`: every
//! sample retained, plus min / median / max) or an engine counter read from
//! a typed outcome (`WorkbookRecalcOutcome::cells_evaluated`, a `Calculated
//! { tick_id }` provenance). What the engine does NOT expose to the host is
//! recorded as such in [`Observability`], never guessed: the seeded lane's
//! `GridSeededLaneOutcome` and the grid M-2 phase timers have no public
//! accessor on `OxCalcDocumentContext` today, so the artifact carries
//! `lane_outcome: null` with the reason, and the committed baseline doc
//! states the code-reading expectation separately from the measurement.
//!
//! W012-1.7 (dtc-5zet.14) added the recalc accounting beside the numbers —
//! per subject `escalation` (the session's `RecalcAccounting` at the end of
//! its series) and run-wide `escalation` (the fold) — additive,
//! `#[serde(default)]`, so a `bench.v1.json` written before it still reads
//! back. See [`crate::escalation`].

use serde::{Deserialize, Serialize};

use crate::escalation::EscalationSummary;

/// The schema tag every bench artifact carries.
pub const BENCH_SCHEMA: &str = "dnacalc-conform/bench.v1";

/// The artifact's file name inside `--out`.
pub const BENCH_ARTIFACT_FILE_NAME: &str = "bench.v1.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchReport {
    pub schema: String,
    pub machine: MachineIdentity,
    pub run: BenchRunSettings,
    pub observability: Observability,
    pub selection: Selection,
    pub subjects: Vec<SubjectReport>,
    /// Every subject's recalc accounting folded (W012-1.7): the run's
    /// `escalation_rate` (engine-reported; `null` while OxCalc exposes no
    /// lane outcome) and its lower bound from the verbs' contract rebuilds.
    #[serde(default)]
    pub escalation: EscalationSummary,
    /// The process's peak working set at the end of the run, when the
    /// platform probe succeeded. Process-wide: OxDoc parses, every subject,
    /// the revision retention the product path keeps per edit, and the
    /// artifact assembly all count. Not an allocation counter.
    pub peak_working_set_bytes: Option<u64>,
    /// Where `peak_working_set_bytes` came from, or why it is absent.
    pub peak_working_set_source: String,
}

/// Who ran the bench, on what. Probed at run time; every field that could
/// not be probed is `None` and named in `probe_failures`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineIdentity {
    pub os: String,
    pub arch: String,
    pub family: String,
    pub logical_cpus: Option<u64>,
    /// `PROCESSOR_IDENTIFIER` (Windows).
    pub processor_identifier: Option<String>,
    /// `Win32_Processor.Name` (Windows, via PowerShell CIM).
    pub cpu_name: Option<String>,
    /// `Win32_ComputerSystem.TotalPhysicalMemory` (Windows, via PowerShell CIM).
    pub total_physical_memory_bytes: Option<u64>,
    /// `rustc -vV` of the `rustc` on `PATH` at bench time (the toolchain
    /// cargo used unless a toolchain override was in force), joined lines.
    pub rustc_version: Option<String>,
    /// `false` under `--release` — the proof the bench binary was an
    /// optimized build. The workspace's release profile is recorded in the
    /// baseline doc verbatim (`opt-level`, `lto`, `codegen-units`).
    pub debug_assertions: bool,
    pub probe_failures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchRunSettings {
    pub corpus_dir: String,
    pub out_dir: String,
    /// Seconds since the Unix epoch when the run started.
    pub started_unix_secs: u64,
    pub total_wall_ms: f64,
    /// How many corpus workbooks the bench measures (the largest by formula
    /// cells that open on the product path).
    pub corpus_subjects: u64,
    /// Rows of the synthetic fill-down (formula cells = rows).
    pub filldown_rows: u64,
    /// Rows of the synthetic cross-sheet workbook (cross-sheet formula cells
    /// = rows).
    pub cross_sheet_rows: u64,
    pub open_repeats: u64,
    pub edit_repeats: u64,
    /// Cycles of `set_calc_mode(Manual)` → edit → `recalculate()` (the F9
    /// verb) per subject; the first cycle on a synthetic subject is its cold
    /// full recalc.
    pub full_recalc_repeats: u64,
    /// The engine validation mode host-core constructs the context with;
    /// recorded because the first recalc of every rebuilt derived state runs
    /// the reference twin under it (see the baseline doc).
    pub validation_mode: String,
}

/// What the host can and cannot observe about the engine's recalc, stated
/// once per artifact so a reader never mistakes an absent number for a
/// measured zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observability {
    /// Whether the seeded lane's `GridSeededLaneOutcome` for an edit could be
    /// read through a public accessor. `false` today: `last_lane_outcome`
    /// is a private field of OxCalc's `GridDerivedState`.
    pub lane_outcome_exposed: bool,
    /// Whether OxCalc's grid M-2 phase timers (`GridRecalcPhaseTimings`)
    /// reach the host. `false` today: they stay inside the machine's recalc
    /// report, which `GridDerivedState::recalc` consumes.
    pub phase_timers_exposed: bool,
    /// Whether `cells_evaluated` is exposed for an Automatic-mode inline
    /// edit. `false` today: only `recalculate_workbook` (F9) returns a
    /// `WorkbookRecalcOutcome`; `enter_grid_cell` returns a view.
    pub edit_cells_evaluated_exposed: bool,
    /// Why there is no allocation counter.
    pub allocation_counter: String,
    pub notes: Vec<String>,
}

/// How the corpus subjects were chosen: every readable workbook ranked by
/// formula cells, Excel-authored ones first, opened in rank order until
/// `corpus_subjects` opened on the product path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Selection {
    pub workbooks_scanned: u64,
    pub unreadable: u64,
    /// The ranked candidates (Excel-authored, formula cells > 0), best first.
    pub ranked: Vec<Candidate>,
    /// Paths chosen, in rank order.
    pub chosen: Vec<String>,
    /// Higher-ranked candidates the product path could not open, with the
    /// typed error kind (`diff::error_kind`).
    pub skipped_open_failed: Vec<SkippedCandidate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub path: String,
    pub formula_cells: u64,
    pub cells: u64,
    pub sheets: u64,
    pub excel_authored: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkippedCandidate {
    pub path: String,
    pub formula_cells: u64,
    pub open_error_kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubjectReport {
    pub id: String,
    pub kind: SubjectKind,
    pub sheets: u64,
    pub cells: u64,
    pub formula_cells: u64,
    /// Corpus subjects only: the open timings.
    pub open: Option<OpenTimings>,
    /// Synthetic subjects only: authoring the workbook through
    /// `enter_grid_cell` under `CalcMode::Manual`.
    pub setup: Option<SetupTimings>,
    /// The Automatic-mode single-cell edit series.
    pub edit: EditTimings,
    /// The Manual-edit + F9 cycles, in order.
    pub full_recalc: Vec<FullRecalcSample>,
    /// The subject's session accounting at the end of its series (W012-1.7):
    /// the open-recalc (corpus subjects), the Manual setup verbs (synthetic
    /// subjects), the `edit_repeats` Automatic entries, the F9 drains — each
    /// a contract rebuild — and the lane-outcome gap.
    #[serde(default)]
    pub escalation: EscalationSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubjectKind {
    Corpus {
        path: String,
        sha256: String,
        excel_authored: bool,
        /// The file's `<calcPr calcMode>`.
        calc_mode: Option<String>,
    },
    Synthetic {
        shape: String,
    },
}

/// A wall-clock sample set. Every sample is retained (milliseconds, `f64`)
/// so a later reader can re-derive any statistic; min / median / max are
/// precomputed for the summary and the baseline doc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Samples {
    pub samples_ms: Vec<f64>,
    pub min_ms: f64,
    pub median_ms: f64,
    pub max_ms: f64,
}

impl Samples {
    /// Build from raw samples; an empty set is all zeros (never produced by
    /// the runner, which always takes at least one sample).
    #[must_use]
    pub fn from_ms(samples_ms: Vec<f64>) -> Self {
        let mut sorted = samples_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let (min_ms, median_ms, max_ms) = if sorted.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            let mid = sorted.len() / 2;
            let median = if sorted.len().is_multiple_of(2) {
                (sorted[mid - 1] + sorted[mid]) / 2.0
            } else {
                sorted[mid]
            };
            (sorted[0], median, sorted[sorted.len() - 1])
        };
        Self {
            samples_ms,
            min_ms,
            median_ms,
            max_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenTimings {
    /// Engine-free OxDoc open (`open_host_owned_xlsx_source`, `LoadProfile::full()`).
    pub oxdoc_only: Samples,
    /// The product path: `WorkbookSession::open_xlsx_bytes` = OxDoc open +
    /// OxCalc ingest + the calc-mode-conditional open-recalc. For an
    /// `Automatic` file this is where the cold full recalc happens.
    pub product_open: Samples,
    /// `product_open.median_ms - oxdoc_only.median_ms`: ingest plus the
    /// open-recalc, derived, not a phase timer.
    pub ingest_plus_open_recalc_median_ms: f64,
    /// `WorkbookLoadReport::engine_recalcs_at_load` (1 for Automatic, 0 for Manual).
    pub engine_recalcs_at_load: u32,
    /// `WorkbookLoadReport::recalc_path`, `Debug`-rendered.
    pub recalc_path: String,
    pub formulas_bound: u32,
    pub bind_degradations: u32,
    pub not_calc_modeled: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetupTimings {
    /// `enter_grid_cell` calls (literals and formulas).
    pub verb_calls: u64,
    pub wall_ms: f64,
    /// `wall_ms / verb_calls`.
    pub mean_ms_per_verb: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditTimings {
    pub sheet: String,
    /// A1 notation.
    pub address: String,
    pub target: EditTarget,
    /// The texts entered, one per repeat, so every sample is a real change.
    pub entered: Vec<String>,
    /// `enter_grid_cell` wall-clock, one sample per repeat. Includes the
    /// engine's inline Automatic recalc, the cross-sheet propagation, the
    /// revision mint, and the verb's returned post-edit readout of the edited
    /// sheet — everything the product path pays per keystroke commit.
    pub wall: Samples,
    /// The `Calculated { tick_id }` the edited cell carried after each
    /// repeat (`None` when its provenance was not `Calculated`): the proof
    /// each timed edit was drained inline.
    pub post_edit_ticks: Vec<Option<u64>>,
    /// A formula cell that reads the edited cell, when one was found, and
    /// what it published after the last repeat.
    pub dependent: Option<DependentReadback>,
    /// `GridSeededLaneOutcome` of the edit's recalc — `None` while OxCalc
    /// exposes no accessor (see [`Observability::lane_outcome_exposed`]).
    pub lane_outcome: Option<String>,
    pub lane_outcome_note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum EditTarget {
    /// An existing literal cell (a same-address overwrite: topology-preserving).
    ExistingLiteral {
        /// Whether a formula on the workbook was found to reference it (a
        /// best-effort address-token scan of the file's formula texts).
        referenced_by_formula: bool,
        original: String,
    },
    /// The workbook carried no literal cell at all; the edit authored a new
    /// literal at the first empty address of column A (topology growth by
    /// construction — a different edit shape, recorded as such).
    NewLiteralCell,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DependentReadback {
    pub sheet: String,
    pub address: String,
    pub formula: String,
    pub value_before: Option<String>,
    pub value_after: Option<String>,
    pub provenance_after: Option<String>,
}

/// One `set_calc_mode(Manual)` → edit → `recalculate()` cycle. The F9 verb
/// (`recalculate_workbook`) rebuilds every seeded sheet's derived state from
/// authored input — `rebuild_from_input` = build + a whole-sheet mark-all
/// recalc with no retained valuation, whose first tick also runs the
/// reference twin under `DualValidatedSampled` — and THEN re-seeds the
/// accumulated edits and runs a second, seeded recalc over them. The
/// wall-clock covers all of it; `cells_evaluated` covers only the second
/// pass (the engine's `last_recalc_cells_evaluated` at the time the
/// outcome is assembled), so the rebuild's mark-all and its oracle sweep
/// are unaccounted in the counter (measured 2026-09-14: a 200-row
/// fill-down's warm F9 reports `cells_evaluated 2` for an edit whose
/// rebuild evaluated all 400 cells first).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FullRecalcSample {
    /// `true` for a synthetic subject's first F9 — the first evaluation the
    /// workbook ever had (every authored cell is then a seed, so the second
    /// pass and the counter cover the whole sheet).
    pub cold: bool,
    pub wall_ms: f64,
    /// `WorkbookRecalcOutcome::total_cells_evaluated()` — the seeded pass
    /// after the rebuild, see above.
    pub cells_evaluated: u64,
    pub drained_sheets: u64,
    pub tick_id: Option<u64>,
    /// `Err` rendering when the F9 itself failed (then the counters are 0).
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_compute_min_median_max_for_odd_and_even_counts() {
        let odd = Samples::from_ms(vec![3.0, 1.0, 2.0]);
        assert_eq!((odd.min_ms, odd.median_ms, odd.max_ms), (1.0, 2.0, 3.0));
        assert_eq!(odd.samples_ms, vec![3.0, 1.0, 2.0], "raw order retained");
        let even = Samples::from_ms(vec![4.0, 1.0, 3.0, 2.0]);
        assert_eq!((even.min_ms, even.median_ms, even.max_ms), (1.0, 2.5, 4.0));
        let empty = Samples::from_ms(Vec::new());
        assert_eq!(
            (empty.min_ms, empty.median_ms, empty.max_ms),
            (0.0, 0.0, 0.0)
        );
    }
}

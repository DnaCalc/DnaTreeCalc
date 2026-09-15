//! Recalc accounting — per-session counters of every recalc the live host
//! path caused and what the engine let the host observe about it (W012-1.7,
//! dtc-5zet.14; `Foundation/notes/PROGRAM_INVESTIGATION_2026-09-10.md` §1
//! "Make escalation cost something", §4 "Escalation is never free").
//!
//! The rule the campaign sets is that every conservative fallback on the
//! live path is **counted and reported beside coverage**, so that "escalate
//! to a full mark-all" stops being free. Counting needs the engine's own
//! per-recalc fact — OxCalc's `GridSeededLaneOutcome` (`Incremental` or one
//! of five escalation reasons) and its `rebuild_from_input` events — and
//! **OxCalc exposes neither to the host today**: `last_lane_outcome` is a
//! private field of the private `GridDerivedState` (`consumer.rs` ~1611),
//! the rebuild is a private constructor, and the verbs return views and
//! drain lists with no lane fact on them
//! (`docs/handovers/HANDOVER_OXCALC_lane_outcome_and_phase_timers_accessor.md`,
//! OxCalc `calc-1q03.15`, open at the HEAD this was written against).
//!
//! So this module counts exactly what the host CAN observe, keeps the typed
//! gap visible, and never infers a lane outcome:
//!
//! - **Verbs and their public outcomes.** Every entry/clear verb is counted
//!   under the calc mode it ran in (read from the engine before the verb);
//!   every F9 is counted with the engine's own `WorkbookRecalcOutcome`
//!   (drained sheets, seeded-pass `cells_evaluated`); the load path's
//!   Automatic open-recalc is counted from the engine's `WorkbookLoadReport`.
//! - **Rebuilds by verb contract.** Two verbs rebuild every sheet they touch
//!   from authored input and recalculate it in full before anything
//!   incremental happens — OxCalc's `clear_grid_cell` under Automatic
//!   (`consumer.rs` ~5787, "rebuild the derived sheet as a pure function of
//!   the mutated input") and `recalculate_workbook` for every drained sheet
//!   (`consumer.rs` ~8670, the doc comment: "rebuilt as a pure function of
//!   its authored input, recalculated"). [`RecalcAccounting::contract_rebuilds`]
//!   counts those transactions. It is a **lower bound** on the escalations,
//!   read from the verb's documented behaviour at the OxCalc HEAD named
//!   above — if OxCalc makes Delete or F9 incremental (W063-7), the F9
//!   counter test in `dnacalc-conform`'s `bench_tests.rs` (warm F9
//!   `cells_evaluated == 2` on the fill-down) is the canary and this
//!   definition is re-cut against the new contract, never left to drift.
//! - **Lane outcomes, engine-reported only.** [`LaneOutcomeCounters`] has one
//!   typed field per `GridSeededLaneOutcome` variant and one for the gap:
//!   `unobserved`. Today every recalc transaction lands in `unobserved`.
//!   When OxCalc's accessor lands (tracked as dtc-5zet.22), the recording
//!   moves to the engine's fact and [`RecalcAccounting::escalation_rate`]
//!   becomes `Some`.
//!
//! What is NOT counted, on purpose: sheet lifecycle verbs and defined-name
//! writes (they can recalc name-dependent grids through OxCalc's
//! `propagate_tree_node_names_to_grids`; a follow-up bead names them), and
//! the per-sheet fan-out inside one transaction (cross-sheet propagation
//! recalculates sheets the engine does not report — the same gap
//! `WorkbookSession::peer_grid_projections` works around by diffing).

use dnacalc_skin_ir::{
    GridAuthoredKindProjection, LaneOutcomeCountersProjection, RecalcAccountingProjection,
};
use oxcalc_core::consumer::WorkbookRecalcOutcome;
use oxcalc_core::oxdoc_ingest::WorkbookLoadReport;
use oxcalc_core::structural::TreeNodeId;
use oxcalc_core::workbook_settings::CalcMode;

use crate::workbook::{WorkbookSession, WorkbookSessionError};

/// Engine-reported `GridSeededLaneOutcome`s per variant — one entry per
/// recalc transaction in [`RecalcAccounting::recalcs`] — plus the typed gap.
/// Every non-`incremental` variant is an escalation to a full mark-all
/// (OxCalc `grid/machine/differential.rs`, `GridSeededLaneOutcome`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LaneOutcomeCounters {
    /// Rode the incremental path over the dirty cone.
    pub incremental: u64,
    /// No valuation retained from a prior recalc (a fresh or rebuilt backing).
    pub no_retained_valuation: u64,
    /// The retained valuation's basis stamp did not match authored truth.
    pub basis_mismatch: u64,
    /// The retained valuation was a visible-first projection.
    pub not_full_coverage: u64,
    /// The retained valuation had no installed dependency graph.
    pub graph_not_installed: u64,
    /// The edit grew the sheet's authored topology (a new cell, a fill).
    pub topology_growth: u64,
    /// Recalc transactions whose outcome the engine did not expose to the
    /// host — every one today (OxCalc `calc-1q03.15` open). Never a guess:
    /// a transaction is here or in exactly one engine-reported variant.
    pub unobserved: u64,
}

impl LaneOutcomeCounters {
    /// Transactions whose outcome the engine reported (any variant).
    #[must_use]
    pub fn observed(&self) -> u64 {
        self.incremental
            + self.no_retained_valuation
            + self.basis_mismatch
            + self.not_full_coverage
            + self.graph_not_installed
            + self.topology_growth
    }

    /// Engine-reported escalations: every observed outcome that is not
    /// `incremental`.
    #[must_use]
    pub fn escalations(&self) -> u64 {
        self.observed() - self.incremental
    }

    /// `observed + unobserved` — equals [`RecalcAccounting::recalcs`] by
    /// construction.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.observed() + self.unobserved
    }

    #[must_use]
    pub fn projection(&self) -> LaneOutcomeCountersProjection {
        LaneOutcomeCountersProjection {
            incremental: self.incremental,
            no_retained_valuation: self.no_retained_valuation,
            basis_mismatch: self.basis_mismatch,
            not_full_coverage: self.not_full_coverage,
            graph_not_installed: self.graph_not_installed,
            topology_growth: self.topology_growth,
            unobserved: self.unobserved,
        }
    }
}

/// Per-session accounting of every recalc the live host path caused (see
/// the module doc). Read through [`WorkbookSession::recalc_accounting`];
/// projected into the Skin IR as `WorkbookCalcProjection::recalc_accounting`
/// by [`RecalcAccounting::projection`]. Plain counters: the derived rates
/// are methods, so a reader never gets a quotient without its denominator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecalcAccounting {
    /// `1` when the load path ran the Automatic open-recalc (OxCalc's
    /// `commit_workbook_tier_a_load` issues exactly one `recalculate_workbook`
    /// under `CalcMode::Automatic`; a non-zero
    /// `WorkbookLoadReport::engine_recalcs_at_load`), `0` otherwise
    /// (`Manual`, or nothing to drain). Kept OUT of
    /// [`Self::recalcs`]: a fresh backing has no retained valuation, so the
    /// open-recalc is a full evaluation by definition, not a fallback from
    /// an incremental path that existed.
    pub open_recalcs: u64,
    /// Sheets the open-recalc drained (`engine_recalcs_at_load`).
    pub open_recalc_sheet_drains: u64,
    /// Entry verbs — [`WorkbookSession::enter_grid_cell`] with non-empty
    /// text, [`WorkbookSession::set_grid_cell_value`] — applied under
    /// Automatic. Each ran the edited sheet's recalc inline (OxCalc
    /// `apply_grid_edit`: a `Cell` seed, then `recalc`) plus cross-sheet
    /// propagation the engine does not report. Whether that recalc rode
    /// `Incremental` (a same-address overwrite with a basis-current retained
    /// valuation) or escalated (`TopologyGrowth` for a write to an empty
    /// address) is the unexposed lane outcome.
    pub automatic_entries: u64,
    /// Clears — [`WorkbookSession::clear_grid_cell`], empty-text entry — of
    /// an authored or pinned cell under Automatic. Each one rebuilt the sheet's
    /// derived state from authored input and recalculated it in full
    /// (OxCalc `clear_grid_cell`: `rebuild_from_input` + `recalc`, "the
    /// optimized sheet has no in-place removal"). A contract rebuild.
    pub automatic_clears: u64,
    /// Clears of an address with nothing authored and nothing published:
    /// OxCalc's idempotent no-op (nothing authored, no pin), no recalc, no
    /// revision. Decided by [`WorkbookSession::clear_target_is_live`] BEFORE
    /// the verb.
    pub clear_noops: u64,
    /// Entry/clear verbs applied under Manual: authored truth moved, seeds
    /// accumulated, no engine pass (drained by the next F9).
    pub manual_edits: u64,
    /// [`WorkbookSession::recalculate`] (F9) calls that drained at least
    /// one sheet.
    pub f9_drains: u64,
    /// F9 calls that found nothing to drain: no tick, no engine pass.
    pub f9_noops: u64,
    /// F9 calls the engine aborted with a typed error — a cell it could
    /// not evaluate under the F9 path's `Abort` policy (the open-recalc
    /// degrades such a cell per calc-1q03.10; the F9 path does not yet,
    /// OxCalc calc-1q03.19). Observed on the live path 2026-09-15
    /// (`SCN-EK011-FUNCTION-RECOGNITION.xlsx`, `D1 = COPILOT(1)`): the
    /// drain aborts, nothing is published, and the drained sheet's seeds
    /// are consumed — `has_undrained_edits` flips to false and the next F9
    /// is a no-op while the edited cell stays `Stale`. Counted apart:
    /// not a recalc transaction (nothing completed), not a no-op.
    pub f9_failures: u64,
    /// Sheets drained across every F9 (`WorkbookRecalcOutcome::drained.len()`
    /// summed). Each was rebuilt from authored input and recalculated in
    /// full — `rebuild_from_input` = build + mark-all with no retained
    /// valuation — BEFORE the seeded pass the outcome counts.
    pub f9_sheet_drains: u64,
    /// `WorkbookRecalcOutcome::total_cells_evaluated()` summed over F9s:
    /// the seeded pass only; the rebuild's mark-all is not in any counter
    /// the engine exposes (measured in `docs/test-runs/w012/BASELINE_2026-09-15.md`).
    pub f9_cells_evaluated: u64,
    /// Engine-reported lane outcomes per variant, plus the gap.
    pub lane_outcomes: LaneOutcomeCounters,
}

impl RecalcAccounting {
    /// Whether this accounting reads the engine's per-recalc
    /// `GridSeededLaneOutcome` at all — the wiring fact, distinct from the
    /// per-run counts. `false` at the OxCalc HEAD this was written against
    /// (no public accessor; `calc-1q03.15` open): every recalc is recorded
    /// `unobserved`. dtc-5zet.22 flips this when the accessor lands and the
    /// recording moves to the engine's fact. A consumer reads this to tell
    /// "nothing was observed because nothing ran" from "nothing can be
    /// observed yet".
    pub const LANE_OUTCOMES_EXPOSED: bool = false;

    /// The accounting of a session opened from `.xlsx` bytes: the load
    /// path's own open-recalc, from the engine's report; nothing else yet.
    #[must_use]
    pub fn from_load_report(report: &WorkbookLoadReport) -> Self {
        Self {
            open_recalcs: u64::from(report.engine_recalcs_at_load > 0),
            open_recalc_sheet_drains: u64::from(report.engine_recalcs_at_load),
            ..Self::default()
        }
    }

    /// Recalc transactions the host issued after the open — the denominator
    /// of every rate here: `automatic_entries + automatic_clears + f9_drains`.
    #[must_use]
    pub fn recalcs(&self) -> u64 {
        self.automatic_entries + self.automatic_clears + self.f9_drains
    }

    /// Of [`Self::recalcs`], the transactions whose verb rebuilds every
    /// drained sheet from authored input and recalculates it in full by
    /// contract: `automatic_clears + f9_drains`. A lower bound on the
    /// escalations (an Automatic entry may escalate too — unobserved).
    #[must_use]
    pub fn contract_rebuilds(&self) -> u64 {
        self.automatic_clears + self.f9_drains
    }

    /// `escalations / recalcs` from ENGINE-REPORTED outcomes only —
    /// `Some` only when every transaction's outcome was observed and there
    /// was at least one; `None` today, since OxCalc exposes no outcome
    /// (`lane_outcomes.unobserved == recalcs`). Never computed from a guess.
    #[must_use]
    pub fn escalation_rate(&self) -> Option<f64> {
        let recalcs = self.recalcs();
        (recalcs > 0 && self.lane_outcomes.unobserved == 0)
            .then(|| self.lane_outcomes.escalations() as f64 / recalcs as f64)
    }

    /// `contract_rebuilds / recalcs`: the escalation rate's lower bound,
    /// from the verbs' documented behaviour. `None` with no recalcs.
    #[must_use]
    pub fn escalation_rate_lower_bound(&self) -> Option<f64> {
        let recalcs = self.recalcs();
        (recalcs > 0).then(|| self.contract_rebuilds() as f64 / recalcs as f64)
    }

    /// The Skin IR mirror: every counter plus the two sums, typed.
    #[must_use]
    pub fn projection(&self) -> RecalcAccountingProjection {
        RecalcAccountingProjection {
            open_recalcs: self.open_recalcs,
            open_recalc_sheet_drains: self.open_recalc_sheet_drains,
            automatic_entries: self.automatic_entries,
            automatic_clears: self.automatic_clears,
            clear_noops: self.clear_noops,
            manual_edits: self.manual_edits,
            f9_drains: self.f9_drains,
            f9_noops: self.f9_noops,
            f9_failures: self.f9_failures,
            f9_sheet_drains: self.f9_sheet_drains,
            f9_cells_evaluated: self.f9_cells_evaluated,
            recalcs: self.recalcs(),
            contract_rebuilds: self.contract_rebuilds(),
            lane_outcomes: self.lane_outcomes.projection(),
        }
    }

    /// One recalc transaction the host issued whose lane outcome the engine
    /// did not expose.
    fn record_unobserved_recalc(&mut self) {
        self.lane_outcomes.unobserved += 1;
    }

    /// An entry verb the engine accepted, under `mode`.
    pub(crate) fn record_entry(&mut self, mode: CalcMode) {
        match mode {
            CalcMode::Automatic => {
                self.automatic_entries += 1;
                self.record_unobserved_recalc();
            }
            CalcMode::Manual => self.manual_edits += 1,
        }
    }

    /// A clear verb the engine accepted, under `mode`; `was_live` is
    /// [`WorkbookSession::clear_target_is_live`] read before the verb —
    /// OxCalc's own `was_authored || was_pinned` no-op test, seen from the
    /// host.
    pub(crate) fn record_clear(&mut self, mode: CalcMode, was_live: bool) {
        if !was_live {
            self.clear_noops += 1;
            return;
        }
        match mode {
            CalcMode::Automatic => {
                self.automatic_clears += 1;
                self.record_unobserved_recalc();
            }
            CalcMode::Manual => self.manual_edits += 1,
        }
    }

    /// An F9 the engine ran, with its outcome.
    pub(crate) fn record_f9(&mut self, outcome: &WorkbookRecalcOutcome) {
        if outcome.drained_any() {
            self.f9_drains += 1;
            self.f9_sheet_drains += outcome.drained.len() as u64;
            self.f9_cells_evaluated += outcome.total_cells_evaluated();
            self.record_unobserved_recalc();
        } else {
            self.f9_noops += 1;
        }
    }

    /// An F9 the engine aborted with a typed error.
    pub(crate) fn record_f9_failure(&mut self) {
        self.f9_failures += 1;
    }
}

// `result_large_err`: same rationale as `workbook.rs`'s impl block.
#[allow(clippy::result_large_err)]
impl WorkbookSession {
    /// The session's recalc accounting so far (W012-1.7): a copy of the
    /// typed counters, read-only.
    #[must_use]
    pub fn recalc_accounting(&self) -> RecalcAccounting {
        self.recalc_accounting
    }

    /// The workbook's calc mode as the engine holds it now — read BEFORE an
    /// entry/clear verb so the verb is counted under the mode it ran in
    /// (OxCalc reads the same setting once, ahead of any mutation).
    pub(crate) fn current_calc_mode(&self) -> Result<CalcMode, WorkbookSessionError> {
        Ok(self
            .context()
            .workbook_calc_settings(self.workspace_id())?
            .calc_mode)
    }

    /// Whether a clear of this address will change anything — OxCalc's
    /// `clear_grid_cell` is an idempotent no-op (no rebuild, no seed, no
    /// revision) unless the address is authored or carries a `FileCached`
    /// pin, and neither fact is readable directly. Read through the public
    /// readouts: the 1x1 authored window (`GridAuthoredKind` other than
    /// `Empty` — a Manual-mode cell authored but never drained has no
    /// published entry yet, so the authored view is asked first), then the
    /// published provenance (a pin publishes its cache; the only other
    /// unauthored published kind, a spill display, is rejected as
    /// non-editable before any mutation and never reaches the accounting).
    pub(crate) fn clear_target_is_live(
        &self,
        sheet: TreeNodeId,
        row: u32,
        col: u32,
    ) -> Result<bool, WorkbookSessionError> {
        let authored = self
            .grid_authored_cells(sheet, row, col, row, col)?
            .iter()
            .any(|cell| {
                cell.row == row && cell.col == col && cell.kind != GridAuthoredKindProjection::Empty
            });
        if authored {
            return Ok(true);
        }
        Ok(self.grid_cell_provenance(sheet, row, col)?.is_some())
    }
}

#[cfg(test)]
mod tests {
    //! The bead's acceptance (dtc-5zet.14): the counters on the W011 fixture
    //! (edit `A1`), on the two-sheet demo, and after a Delete — each with
    //! the lane outcome recorded HONESTLY: `unobserved`, because OxCalc
    //! exposes none. What the code reading says the outcome *should* be is
    //! stated in each test's doc comment as an expectation, never asserted
    //! as an observation.

    use super::*;
    use crate::demo::build_demo_workbook;
    use crate::xlsx_fixture::w011_fixture_bytes;
    use dnacalc_skin_ir::CalcModeProjection;

    fn lane(unobserved: u64) -> LaneOutcomeCounters {
        LaneOutcomeCounters {
            unobserved,
            ..LaneOutcomeCounters::default()
        }
    }

    /// The W011 fixture (`A1 = 7`, `B1 = =A1*3`, `calcMode` automatic):
    /// the open ran the engine's open-recalc (1 sheet); the edit `A1 = 10`
    /// is one Automatic entry = one recalc transaction, no contract
    /// rebuild. Code-reading expectation for that edit's lane outcome: a
    /// same-address overwrite with a basis-current retained valuation rides
    /// `Incremental` (OxCalc `apply_grid_edit` leaves
    /// `topology_grew_since_recalc` false) — NOT observable from the host,
    /// so the counter says `unobserved: 1` and the rate is `None`; the lower
    /// bound is `0/1`.
    #[test]
    fn w011_fixture_open_then_edit_a1_counts_one_unobserved_automatic_entry() {
        let bytes = w011_fixture_bytes();
        let mut session =
            WorkbookSession::open_xlsx_bytes("workbook:w012-1-7-w011", &bytes, None).unwrap();
        let after_open = session.recalc_accounting();
        assert_eq!(
            after_open,
            RecalcAccounting {
                open_recalcs: 1,
                open_recalc_sheet_drains: 1,
                ..RecalcAccounting::default()
            },
            "the Automatic open-recalc drained the one sheet; no host verb yet"
        );
        assert_eq!(after_open.recalcs(), 0);
        assert_eq!(after_open.escalation_rate(), None);
        assert_eq!(after_open.escalation_rate_lower_bound(), None);

        let sheet = session.sheets().unwrap()[0].node_id;
        session.enter_grid_cell(sheet, 1, 1, "10").unwrap();
        let after_edit = session.recalc_accounting();
        assert_eq!(
            after_edit,
            RecalcAccounting {
                open_recalcs: 1,
                open_recalc_sheet_drains: 1,
                automatic_entries: 1,
                lane_outcomes: lane(1),
                ..RecalcAccounting::default()
            }
        );
        assert_eq!(after_edit.recalcs(), 1);
        assert_eq!(after_edit.contract_rebuilds(), 0);
        assert_eq!(
            after_edit.escalation_rate(),
            None,
            "no lane outcome is engine-reported, so no exact rate is claimed"
        );
        assert_eq!(after_edit.escalation_rate_lower_bound(), Some(0.0));
        assert_eq!(after_edit.lane_outcomes.total(), after_edit.recalcs());

        // The projection carries the same counters, typed.
        let projection = session.workbook_calc_projection(None).unwrap();
        assert_eq!(projection.recalc_accounting, Some(after_edit.projection()));
        let p = projection.recalc_accounting.unwrap();
        assert_eq!((p.recalcs, p.contract_rebuilds), (1, 0));
        assert_eq!(p.lane_outcomes.unobserved, 1);
    }

    /// The two-sheet demo is authored through 13 Automatic `enter_grid_cell`
    /// calls (`Sheet1!A1..B5`, `Sheet2!A1..B2`): 13 recalc transactions, no
    /// contract rebuild. Code-reading expectation: every one of those writes
    /// is to a previously-empty address, so each recalc escalates
    /// `TopologyGrowth` (OxCalc `apply_grid_edit` sets
    /// `topology_grew_since_recalc` for a new address). A 14th edit —
    /// `Sheet1!A1 = 9`, a same-address overwrite — should ride `Incremental`
    /// on `Sheet1`, while its cross-sheet propagation to `Sheet2` re-enters
    /// with a mark-all (`recalculate_sheet_with_cross_sheet_view` sets the
    /// flag unconditionally). None of that is observable: 14 `unobserved`.
    #[test]
    fn two_sheet_demo_counts_every_automatic_entry_as_an_unobserved_recalc() {
        let mut session = build_demo_workbook().unwrap();
        let after_build = session.recalc_accounting();
        assert_eq!(
            after_build,
            RecalcAccounting {
                automatic_entries: 13,
                lane_outcomes: lane(13),
                ..RecalcAccounting::default()
            },
            "an in-memory workbook has no open-recalc; 13 Automatic entries"
        );

        let sheet1 = session.sheets().unwrap()[0].node_id;
        session.enter_grid_cell(sheet1, 1, 1, "9").unwrap();
        let sheet2 = session.sheets().unwrap()[1].node_id;
        assert_eq!(
            session.grid_cell_value(sheet2, 1, 1).unwrap(),
            Some(oxfunc_core::value::CalcValue::number(14.0)),
            "sanity: Sheet2!A1 = Sheet1!A1 + Sheet1!A5 = 9 + 5"
        );
        let after_edit = session.recalc_accounting();
        assert_eq!(after_edit.automatic_entries, 14);
        assert_eq!(after_edit.recalcs(), 14);
        assert_eq!(after_edit.contract_rebuilds(), 0);
        assert_eq!(after_edit.lane_outcomes, lane(14));
        assert_eq!(after_edit.escalation_rate(), None);
        assert_eq!(after_edit.escalation_rate_lower_bound(), Some(0.0));
    }

    /// A Delete (`clear_grid_cell`) of a published cell under Automatic is
    /// a contract rebuild: OxCalc rebuilds the derived sheet from authored
    /// input and recalculates it in full. Code-reading expectation for the
    /// rebuilt sheet's first recalc: `NoRetainedValuation` (the rebuild
    /// starts with no retained valuation) — unobserved from the host, so the
    /// count is `automatic_clears: 1`, `contract_rebuilds: 1`,
    /// `unobserved: 1`. Clearing an address with nothing published is the
    /// engine's idempotent no-op and is counted as such, not as a recalc;
    /// an empty-text entry takes the clear path too.
    #[test]
    fn delete_under_automatic_is_a_contract_rebuild_and_an_empty_clear_is_a_noop() {
        let bytes = w011_fixture_bytes();
        let mut session =
            WorkbookSession::open_xlsx_bytes("workbook:w012-1-7-delete", &bytes, None).unwrap();
        let sheet = session.sheets().unwrap()[0].node_id;

        // Nothing published at D9: the engine's no-op.
        assert_eq!(session.grid_cell_provenance(sheet, 9, 4).unwrap(), None);
        session.clear_grid_cell(sheet, 9, 4).unwrap();
        let after_noop = session.recalc_accounting();
        assert_eq!(after_noop.clear_noops, 1);
        assert_eq!(after_noop.automatic_clears, 0);
        assert_eq!(after_noop.recalcs(), 0);

        // A1 is published (the open-recalc calculated it): a real Delete.
        assert!(session.grid_cell_provenance(sheet, 1, 1).unwrap().is_some());
        session.clear_grid_cell(sheet, 1, 1).unwrap();
        let after_delete = session.recalc_accounting();
        assert_eq!(
            after_delete,
            RecalcAccounting {
                open_recalcs: 1,
                open_recalc_sheet_drains: 1,
                automatic_clears: 1,
                clear_noops: 1,
                lane_outcomes: lane(1),
                ..RecalcAccounting::default()
            }
        );
        assert_eq!(after_delete.recalcs(), 1);
        assert_eq!(after_delete.contract_rebuilds(), 1);
        assert_eq!(after_delete.escalation_rate_lower_bound(), Some(1.0));
        assert_eq!(after_delete.escalation_rate(), None);

        // An empty-text entry takes the clear path: B1 (`=A1*3`) is still
        // published — it recalculated against the now-empty A1 — so this is
        // a second real Delete, not a no-op.
        assert!(session.grid_cell_provenance(sheet, 1, 2).unwrap().is_some());
        session.enter_grid_cell(sheet, 1, 2, "").unwrap();
        let after_empty_entry = session.recalc_accounting();
        assert_eq!(after_empty_entry.automatic_clears, 2);
        assert_eq!(after_empty_entry.contract_rebuilds(), 2);
        assert_eq!(after_empty_entry.recalcs(), 2);
    }

    /// Manual mode: entries and clears accumulate seeds without an engine
    /// pass (`manual_edits`), the F9 that drains them is one recalc
    /// transaction — every drained sheet a contract rebuild, the seeded
    /// pass's `cells_evaluated` retained — and a repeat F9 is a no-op.
    #[test]
    fn manual_edits_then_f9_count_the_drain_and_a_repeat_f9_is_a_noop() {
        let mut session = WorkbookSession::create("workbook:w012-1-7-manual").unwrap();
        let sheet = session.add_sheet("Sheet1").unwrap();
        session.enter_grid_cell(sheet, 1, 1, "7").unwrap();
        session.enter_grid_cell(sheet, 1, 2, "=A1*3").unwrap();
        assert_eq!(session.recalc_accounting().automatic_entries, 2);

        session.set_calc_mode(CalcModeProjection::Manual).unwrap();
        session.enter_grid_cell(sheet, 1, 1, "10").unwrap();
        session.enter_grid_cell(sheet, 2, 1, "1").unwrap();
        session.clear_grid_cell(sheet, 2, 1).unwrap();
        let before_f9 = session.recalc_accounting();
        assert_eq!(
            before_f9.manual_edits, 3,
            "two entries and a clear, no engine pass"
        );
        assert_eq!(
            before_f9.clear_noops, 0,
            "A2 was authored under Manual and never drained (no published entry): the              authored readout, not the published one, decided the clear was live"
        );
        assert_eq!(before_f9.recalcs(), 2, "still the two Automatic entries");
        assert_eq!(before_f9.f9_drains, 0);

        let outcome = session.recalculate().unwrap();
        assert!(outcome.drained_any());
        let after_f9 = session.recalc_accounting();
        assert_eq!(after_f9.f9_drains, 1);
        assert_eq!(after_f9.f9_sheet_drains, 1);
        assert_eq!(
            after_f9.f9_cells_evaluated,
            outcome.total_cells_evaluated(),
            "the seeded pass's counter, as the engine reported it"
        );
        assert_eq!(after_f9.recalcs(), 3);
        assert_eq!(after_f9.contract_rebuilds(), 1);
        assert_eq!(after_f9.lane_outcomes, lane(3));
        assert_eq!(after_f9.escalation_rate_lower_bound(), Some(1.0 / 3.0));

        let repeat = session.recalculate().unwrap();
        assert!(!repeat.drained_any());
        let after_repeat = session.recalc_accounting();
        assert_eq!(after_repeat.f9_noops, 1);
        assert_eq!(after_repeat.f9_drains, 1);
        assert_eq!(after_repeat.recalcs(), 3, "a no-op F9 is not a recalc");
    }

    /// An F9 the engine aborts is counted as a failure, apart from drains
    /// and no-ops: under Manual, `=COPILOT(1)` (an unknown callable — OxFml
    /// binds it, the runtime cannot evaluate it) accumulates a seed, and the
    /// F9 that drains it fails typed under the F9 path's `Abort` policy.
    /// Recorded here as observed 2026-09-15: the failed F9 consumed the
    /// seed — the sheet reads clean afterwards and a second F9 is a no-op,
    /// while the edited cell keeps its `Stale` publication (the sibling
    /// finding dtc-5zet.14 filed; OxCalc calc-1q03.19 owns the F9 path).
    #[test]
    fn a_failed_f9_is_counted_apart_and_drops_the_seed() {
        let mut session = WorkbookSession::create("workbook:w012-1-7-f9-failure").unwrap();
        let sheet = session.add_sheet("Sheet1").unwrap();
        session.enter_grid_cell(sheet, 1, 1, "1").unwrap();
        session.set_calc_mode(CalcModeProjection::Manual).unwrap();
        session.enter_grid_cell(sheet, 1, 2, "=COPILOT(1)").unwrap();
        assert!(
            session.workbook_calc_projection(None).unwrap().sheets[0].dirty,
            "the Manual entry accumulated a seed"
        );
        let failed = session.recalculate();
        assert!(
            matches!(&failed, Err(WorkbookSessionError::OxCalc(_))),
            "the F9 aborts typed on the unevaluable cell: {failed:?}"
        );
        let after = session.recalc_accounting();
        assert_eq!(
            (after.f9_failures, after.f9_drains, after.f9_noops),
            (1, 0, 0)
        );
        assert_eq!(
            after.recalcs(),
            1,
            "the one Automatic entry (A1); a failed F9 is not a recalc"
        );
        assert_eq!(after.lane_outcomes.total(), after.recalcs());
        // The observed consequence, pinned as found (not as wanted): the
        // seed is gone and nothing drained it.
        assert!(
            !session.workbook_calc_projection(None).unwrap().sheets[0].dirty,
            "OBSERVED: a failed F9 consumes the sheet's seeds — the sheet reads clean"
        );
        let repeat = session.recalculate().unwrap();
        assert!(
            !repeat.drained_any(),
            "OBSERVED: the next F9 finds nothing to drain"
        );
        assert_eq!(session.recalc_accounting().f9_noops, 1);
    }

    /// A rejected entry (`=1+`, OxFml diagnostics) mutates nothing in the
    /// engine and is counted nowhere.
    #[test]
    fn a_rejected_entry_is_not_counted() {
        let mut session = WorkbookSession::create("workbook:w012-1-7-rejected").unwrap();
        let sheet = session.add_sheet("Sheet1").unwrap();
        assert!(session.enter_grid_cell(sheet, 1, 1, "=1+").is_err());
        assert_eq!(session.recalc_accounting(), RecalcAccounting::default());
    }

    #[test]
    fn lane_outcome_counters_sum_observed_escalations_and_total() {
        let counters = LaneOutcomeCounters {
            incremental: 3,
            no_retained_valuation: 1,
            basis_mismatch: 0,
            not_full_coverage: 0,
            graph_not_installed: 0,
            topology_growth: 2,
            unobserved: 4,
        };
        assert_eq!(counters.observed(), 6);
        assert_eq!(counters.escalations(), 3);
        assert_eq!(counters.total(), 10);
        // An exact rate needs every transaction observed.
        let partly = RecalcAccounting {
            automatic_entries: 10,
            lane_outcomes: counters,
            ..RecalcAccounting::default()
        };
        assert_eq!(partly.escalation_rate(), None);
        let fully = RecalcAccounting {
            automatic_entries: 6,
            lane_outcomes: LaneOutcomeCounters {
                unobserved: 0,
                ..counters
            },
            ..RecalcAccounting::default()
        };
        assert_eq!(fully.escalation_rate(), Some(0.5));
    }
}

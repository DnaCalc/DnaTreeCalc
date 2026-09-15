//! `escalation` — the instrument's view of host-core's recalc accounting
//! (W012-1.7, dtc-5zet.14): what every `diff` workbook and every `bench`
//! subject retains beside coverage, and the corpus-wide fold.
//!
//! The Foundation note's rule is that every conservative fallback on the
//! live path is counted and reported beside coverage, so escalation stops
//! being free. The count comes from `dnacalc_host_core::RecalcAccounting`,
//! which records every recalc the session's verbs caused. What it CAN say
//! today and what it cannot is stated in the summary itself, typed:
//!
//! - `escalation_rate` is the engine-reported escalations over the recalcs
//!   and is `null` while any recalc's `GridSeededLaneOutcome` is unobserved
//!   — which is every recalc, because OxCalc exposes no lane outcome to the
//!   host (`lane_outcomes.unobserved == recalcs`; OxCalc `calc-1q03.15`,
//!   `docs/handovers/HANDOVER_OXCALC_lane_outcome_and_phase_timers_accessor.md`);
//! - `escalation_rate_lower_bound` is `contract_rebuilds / recalcs`: the
//!   transactions whose verb rebuilds every drained sheet from authored
//!   input and recalculates it in full by contract (an Automatic Delete,
//!   every F9 drain), over the recalcs the host issued. A true escalation
//!   rate is never below it; when the accessor lands the exact rate takes
//!   over and the bound stays as a cross-check.
//!
//! Nothing here is inferred from the engine's behaviour: every field is a
//! host-core counter or a quotient of two of them.

use std::io::{self, Write};

use dnacalc_host_core::{LaneOutcomeCounters, RecalcAccounting};
use serde::{Deserialize, Serialize};

/// One session's recalc accounting, as retained in an artifact. Every
/// counter is `RecalcAccounting`'s (see its field docs); the two rates are
/// its methods, `None` where undefined.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct EscalationSummary {
    /// Recalc transactions the host issued after the open:
    /// `automatic_entries + automatic_clears + f9_drains`. The denominator
    /// of both rates.
    pub recalcs: u64,
    /// Transactions whose verb rebuilds every drained sheet from authored
    /// input and recalculates it in full (OxCalc `clear_grid_cell` under
    /// Automatic; `recalculate_workbook` per drained sheet): a lower bound
    /// on the escalations, counted from the verb's contract.
    pub contract_rebuilds: u64,
    /// Engine-reported escalations over `recalcs` — `None` while any lane
    /// outcome is unobserved (every one today) or with no recalcs.
    pub escalation_rate: Option<f64>,
    /// `contract_rebuilds / recalcs`; `None` with no recalcs.
    pub escalation_rate_lower_bound: Option<f64>,
    /// Whether host-core's accounting reads the engine's lane outcome at
    /// all (`RecalcAccounting::LANE_OUTCOMES_EXPOSED`, the wiring fact) —
    /// `false` today. Distinct from the counts: a workbook with 0 recalcs
    /// has 0 unobserved outcomes and still cannot observe any.
    pub lane_outcomes_exposed: bool,
    /// The load path's Automatic open-recalc (`1`/`0`) and the sheets it
    /// drained. Kept out of `recalcs`: a fresh backing has no retained
    /// valuation, so the open-recalc is a full evaluation by definition.
    pub open_recalcs: u64,
    pub open_recalc_sheet_drains: u64,
    pub automatic_entries: u64,
    pub automatic_clears: u64,
    pub clear_noops: u64,
    pub manual_edits: u64,
    pub f9_drains: u64,
    pub f9_noops: u64,
    /// F9s the engine aborted with a typed error (the F9 path's `Abort`
    /// policy on a cell it cannot evaluate); nothing published, the seeds
    /// consumed. Neither a recalc nor a no-op.
    #[serde(default)]
    pub f9_failures: u64,
    /// Sheets drained across every F9, each a contract rebuild.
    pub f9_sheet_drains: u64,
    /// The F9s' seeded-pass `cells_evaluated`, summed (never the rebuild's
    /// mark-all, which the engine does not count).
    pub f9_cells_evaluated: u64,
    pub lane_outcomes: LaneOutcomeCounts,
}

/// `LaneOutcomeCounters`, retained: one field per `GridSeededLaneOutcome`
/// variant plus `unobserved`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LaneOutcomeCounts {
    pub incremental: u64,
    pub no_retained_valuation: u64,
    pub basis_mismatch: u64,
    pub not_full_coverage: u64,
    pub graph_not_installed: u64,
    pub topology_growth: u64,
    pub unobserved: u64,
}

impl LaneOutcomeCounts {
    #[must_use]
    pub fn from_counters(counters: &LaneOutcomeCounters) -> Self {
        Self {
            incremental: counters.incremental,
            no_retained_valuation: counters.no_retained_valuation,
            basis_mismatch: counters.basis_mismatch,
            not_full_coverage: counters.not_full_coverage,
            graph_not_installed: counters.graph_not_installed,
            topology_growth: counters.topology_growth,
            unobserved: counters.unobserved,
        }
    }

    /// Engine-reported outcomes, any variant.
    #[must_use]
    pub fn observed(&self) -> u64 {
        self.incremental
            + self.no_retained_valuation
            + self.basis_mismatch
            + self.not_full_coverage
            + self.graph_not_installed
            + self.topology_growth
    }

    /// Engine-reported escalations (observed, not `incremental`).
    #[must_use]
    pub fn escalations(&self) -> u64 {
        self.observed() - self.incremental
    }

    #[must_use]
    pub fn total(&self) -> u64 {
        self.observed() + self.unobserved
    }

    fn add(&mut self, other: &Self) {
        self.incremental += other.incremental;
        self.no_retained_valuation += other.no_retained_valuation;
        self.basis_mismatch += other.basis_mismatch;
        self.not_full_coverage += other.not_full_coverage;
        self.graph_not_installed += other.graph_not_installed;
        self.topology_growth += other.topology_growth;
        self.unobserved += other.unobserved;
    }
}

impl EscalationSummary {
    /// Retain a session's accounting.
    #[must_use]
    pub fn from_accounting(accounting: &RecalcAccounting) -> Self {
        Self {
            recalcs: accounting.recalcs(),
            contract_rebuilds: accounting.contract_rebuilds(),
            escalation_rate: accounting.escalation_rate(),
            escalation_rate_lower_bound: accounting.escalation_rate_lower_bound(),
            lane_outcomes_exposed: RecalcAccounting::LANE_OUTCOMES_EXPOSED,
            open_recalcs: accounting.open_recalcs,
            open_recalc_sheet_drains: accounting.open_recalc_sheet_drains,
            automatic_entries: accounting.automatic_entries,
            automatic_clears: accounting.automatic_clears,
            clear_noops: accounting.clear_noops,
            manual_edits: accounting.manual_edits,
            f9_drains: accounting.f9_drains,
            f9_noops: accounting.f9_noops,
            f9_failures: accounting.f9_failures,
            f9_sheet_drains: accounting.f9_sheet_drains,
            f9_cells_evaluated: accounting.f9_cells_evaluated,
            lane_outcomes: LaneOutcomeCounts::from_counters(&accounting.lane_outcomes),
        }
    }

    /// Sum the counters of many summaries and recompute the rates over the
    /// sums (corpus-wide or run-wide): `escalation_rate` stays `None` if any
    /// contributor had an unobserved outcome.
    #[must_use]
    pub fn fold<'a>(summaries: impl IntoIterator<Item = &'a Self>) -> Self {
        let mut total = Self::default();
        for summary in summaries {
            total.recalcs += summary.recalcs;
            total.contract_rebuilds += summary.contract_rebuilds;
            total.open_recalcs += summary.open_recalcs;
            total.open_recalc_sheet_drains += summary.open_recalc_sheet_drains;
            total.automatic_entries += summary.automatic_entries;
            total.automatic_clears += summary.automatic_clears;
            total.clear_noops += summary.clear_noops;
            total.manual_edits += summary.manual_edits;
            total.f9_drains += summary.f9_drains;
            total.f9_noops += summary.f9_noops;
            total.f9_failures += summary.f9_failures;
            total.f9_sheet_drains += summary.f9_sheet_drains;
            total.f9_cells_evaluated += summary.f9_cells_evaluated;
            total.lane_outcomes.add(&summary.lane_outcomes);
        }
        total.lane_outcomes_exposed = RecalcAccounting::LANE_OUTCOMES_EXPOSED;
        total.escalation_rate = (total.recalcs > 0 && total.lane_outcomes.unobserved == 0)
            .then(|| total.lane_outcomes.escalations() as f64 / total.recalcs as f64);
        total.escalation_rate_lower_bound =
            (total.recalcs > 0).then(|| total.contract_rebuilds as f64 / total.recalcs as f64);
        total
    }

    /// The identities every retained summary must satisfy — checked before
    /// an artifact is written, like the coverage identities:
    /// `recalcs == automatic_entries + automatic_clears + f9_drains`,
    /// `contract_rebuilds == automatic_clears + f9_drains`, and
    /// `lane_outcomes total == recalcs`.
    pub fn check_identities(&self, what: &str) -> Result<(), String> {
        if self.recalcs != self.automatic_entries + self.automatic_clears + self.f9_drains {
            return Err(format!(
                "{what}: recalcs {} != automatic_entries {} + automatic_clears {} + f9_drains {}",
                self.recalcs, self.automatic_entries, self.automatic_clears, self.f9_drains
            ));
        }
        if self.contract_rebuilds != self.automatic_clears + self.f9_drains {
            return Err(format!(
                "{what}: contract_rebuilds {} != automatic_clears {} + f9_drains {}",
                self.contract_rebuilds, self.automatic_clears, self.f9_drains
            ));
        }
        if self.lane_outcomes.total() != self.recalcs {
            return Err(format!(
                "{what}: lane outcomes total {} != recalcs {}",
                self.lane_outcomes.total(),
                self.recalcs
            ));
        }
        Ok(())
    }

    /// The one-line stdout rendering, printed beside coverage: the rate (or
    /// why it is unobservable), the lower bound with its numerator and
    /// denominator, the verb counts, and the open-recalc kept apart.
    pub fn write_line(&self, label: &str, out: &mut impl Write) -> io::Result<()> {
        write!(out, "{label} ")?;
        match self.escalation_rate {
            Some(rate) => write!(
                out,
                "escalation_rate {rate:.4} = escalations {} / recalcs {}",
                self.lane_outcomes.escalations(),
                self.recalcs
            )?,
            None if self.recalcs == 0 => write!(out, "escalation_rate n/a (0 recalcs)")?,
            None => write!(
                out,
                "escalation_rate UNOBSERVABLE (lane outcomes engine-reported {} / {}; OxCalc \
                 exposes no GridSeededLaneOutcome — calc-1q03.15)",
                self.lane_outcomes.observed(),
                self.recalcs
            )?,
        }
        match self.escalation_rate_lower_bound {
            Some(bound) => write!(
                out,
                "; lower bound {bound:.4} = contract_rebuilds {} / recalcs {}",
                self.contract_rebuilds, self.recalcs
            )?,
            None => write!(out, "; lower bound n/a")?,
        }
        writeln!(
            out,
            "; verbs: automatic_entries {} automatic_clears {} f9_drains {} (sheets {}, cells_evaluated {}) f9_noops {} f9_failures {} manual_edits {} clear_noops {}; open_recalcs {} (sheets {}, not in the rate)",
            self.automatic_entries,
            self.automatic_clears,
            self.f9_drains,
            self.f9_sheet_drains,
            self.f9_cells_evaluated,
            self.f9_noops,
            self.f9_failures,
            self.manual_edits,
            self.clear_noops,
            self.open_recalcs,
            self.open_recalc_sheet_drains
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(recalcs_split: (u64, u64, u64), unobserved: u64) -> EscalationSummary {
        let (entries, clears, f9s) = recalcs_split;
        EscalationSummary::from_accounting(&RecalcAccounting {
            automatic_entries: entries,
            automatic_clears: clears,
            f9_drains: f9s,
            lane_outcomes: LaneOutcomeCounters {
                unobserved,
                ..LaneOutcomeCounters::default()
            },
            ..RecalcAccounting::default()
        })
    }

    #[test]
    fn from_accounting_keeps_the_rate_unobservable_and_computes_the_bound() {
        let s = summary((5, 1, 2), 8);
        assert_eq!(s.recalcs, 8);
        assert_eq!(s.contract_rebuilds, 3);
        assert_eq!(s.escalation_rate, None, "8 of 8 outcomes unobserved");
        assert_eq!(s.escalation_rate_lower_bound, Some(3.0 / 8.0));
        assert!(!s.lane_outcomes_exposed);
        s.check_identities("test").unwrap();

        let empty = summary((0, 0, 0), 0);
        assert_eq!(empty.escalation_rate, None);
        assert_eq!(empty.escalation_rate_lower_bound, None);
        assert!(
            !empty.lane_outcomes_exposed,
            "the wiring fact, not a vacuous truth: nothing ran, and nothing could be observed"
        );
    }

    #[test]
    fn fold_sums_counters_and_recomputes_rates_over_the_sums() {
        let a = summary((5, 1, 2), 8);
        let b = summary((0, 0, 1), 1);
        let c = summary((0, 0, 0), 0);
        let total = EscalationSummary::fold([&a, &b, &c]);
        assert_eq!(total.recalcs, 9);
        assert_eq!(total.contract_rebuilds, 4);
        assert_eq!(total.lane_outcomes.unobserved, 9);
        assert_eq!(total.escalation_rate, None);
        assert_eq!(total.escalation_rate_lower_bound, Some(4.0 / 9.0));
        total.check_identities("fold").unwrap();
    }

    #[test]
    fn an_exact_rate_needs_every_outcome_observed() {
        let observed = EscalationSummary::from_accounting(&RecalcAccounting {
            automatic_entries: 4,
            lane_outcomes: LaneOutcomeCounters {
                incremental: 3,
                topology_growth: 1,
                ..LaneOutcomeCounters::default()
            },
            ..RecalcAccounting::default()
        });
        assert!(
            !observed.lane_outcomes_exposed,
            "the flag is the wiring constant (false until dtc-5zet.22), even on fabricated counts"
        );
        assert_eq!(
            observed.escalation_rate,
            Some(0.25),
            "the rate itself is data-driven: every outcome observed, so it is exact"
        );
        assert_eq!(observed.escalation_rate_lower_bound, Some(0.0));
        observed.check_identities("observed").unwrap();
        // Folding it with an unobserved contributor loses the exact rate.
        let mixed = EscalationSummary::fold([&observed, &summary((1, 0, 0), 1)]);
        assert_eq!(mixed.escalation_rate, None);
        assert_eq!(mixed.lane_outcomes.incremental, 3);
    }

    #[test]
    fn identities_reject_a_tampered_summary() {
        let mut s = summary((5, 1, 2), 8);
        s.contract_rebuilds = 4;
        assert!(
            s.check_identities("tampered")
                .unwrap_err()
                .contains("contract_rebuilds")
        );
        let mut s = summary((5, 1, 2), 8);
        s.lane_outcomes.unobserved = 7;
        assert!(
            s.check_identities("tampered")
                .unwrap_err()
                .contains("lane outcomes")
        );
    }

    #[test]
    fn write_line_names_the_gap_and_the_bound() {
        let mut out = Vec::new();
        summary((5, 1, 2), 8)
            .write_line("ESCALATION", &mut out)
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        assert!(
            line.starts_with("ESCALATION escalation_rate UNOBSERVABLE"),
            "{line}"
        );
        assert!(line.contains("engine-reported 0 / 8"), "{line}");
        assert!(
            line.contains("lower bound 0.3750 = contract_rebuilds 3 / recalcs 8"),
            "{line}"
        );
        let mut out = Vec::new();
        summary((0, 0, 0), 0)
            .write_line("ESCALATION", &mut out)
            .unwrap();
        assert!(String::from_utf8(out).unwrap().contains("n/a (0 recalcs)"));
    }
}

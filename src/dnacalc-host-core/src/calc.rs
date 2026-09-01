//! Calc mode + provenance + recalc — the host-core fill of the Skin IR's
//! calc-state projection (H5, `FRONTEND_UI_DESIGN_AND_ROUTEMAP.md` §A.2/§A.3)
//! from OxCalc's `workbook_calc_settings`/`set_workbook_calc_settings` and
//! `recalculate_workbook` (W062 R5.6, D4 §6).
//!
//! `WorkspaceState::workbook_calc` (skin-IR `workspace.rs`) is a serde-clean
//! mirror of OxCalc's `WorkbookCalcSettings` plus the last recalc tick and a
//! per-sheet dirty summary, and `GridCellProjection::provenance` mirrors
//! OxCalc's `PublishedValueProvenance` per cell — the fact that decides
//! whether a published value is genuinely fresh (`Calculated`), honestly
//! stale behind undrained edits (`Stale`), or a pre-engine file cache
//! (`FileCached`, R6-unpopulated). Neither field is a value fact: switching
//! calc mode never invalidates a computed value (D1 §5), and reading
//! provenance never mutates anything.

use dnacalc_skin_ir::{
    CalcModeProjection, SheetCalcSummaryProjection, ValueProvenanceProjection,
    WorkbookCalcProjection,
};
use oxcalc_core::consumer::WorkbookRecalcOutcome;
use oxcalc_core::workbook_settings::{CalcMode, PublishedValueProvenance, WorkbookCalcSettings};

use crate::workbook::{WorkbookSession, WorkbookSessionError};

/// Project OxCalc's [`CalcMode`] into its skin-IR mirror.
#[must_use]
pub fn calc_mode_projection(mode: CalcMode) -> CalcModeProjection {
    match mode {
        CalcMode::Automatic => CalcModeProjection::Automatic,
        CalcMode::Manual => CalcModeProjection::Manual,
    }
}

/// Resolve a skin-IR [`CalcModeProjection`] back to OxCalc's own [`CalcMode`]
/// (the intent-level write direction — skins never compose the engine's own
/// wire-text encoding themselves, §A.2).
#[must_use]
pub fn calc_mode_from_projection(mode: CalcModeProjection) -> CalcMode {
    match mode {
        CalcModeProjection::Automatic => CalcMode::Automatic,
        CalcModeProjection::Manual => CalcMode::Manual,
    }
}

/// Project OxCalc's [`PublishedValueProvenance`] into its skin-IR mirror.
#[must_use]
pub fn value_provenance_projection(
    provenance: PublishedValueProvenance,
) -> ValueProvenanceProjection {
    match provenance {
        PublishedValueProvenance::Calculated { tick_id } => {
            ValueProvenanceProjection::Calculated { tick_id }
        }
        PublishedValueProvenance::Stale { since_tick_id } => {
            ValueProvenanceProjection::Stale { since_tick_id }
        }
        PublishedValueProvenance::FileCached => ValueProvenanceProjection::FileCached,
        // W062 R6.2 added `Degraded` (a cache-less ingest-degraded `#NAME?`).
        // The host renders it like a retained, non-engine value for now; a
        // distinct degraded affordance is front-end (R7) work, not modeled here.
        PublishedValueProvenance::Degraded => ValueProvenanceProjection::FileCached,
    }
}

// `result_large_err`: same rationale as `workbook.rs`'s impl block and
// `defined_names.rs` — this module's errors wrap the same
// `WorkbookSessionError`/engine error by value for cross-module consistency.
#[allow(clippy::result_large_err)]
impl WorkbookSession {
    /// The workbook's calc-mode/recalc projection (H5, §A.3): mode, the last
    /// recalc's tick (if any drain has run under this session so far — H5
    /// does not persist this across a session reload, matching the engine's
    /// own session-local `last_result` convention elsewhere), and a per-sheet
    /// dirty summary read from the engine's own undrained-edits accounting
    /// (`has_undrained_edits`, W062 hotfix calc-5kqg.57) rather than scanned
    /// from published cell provenance. A "any published cell Stale" scan is a
    /// false negative for a brand-new cell under Manual mode: an address with
    /// no prior published entry has nothing for the engine's own
    /// `mark_published_stale` to re-tag, so the grid view can show zero
    /// `Stale` cells even though an edit is genuinely undrained. The engine
    /// readout has no such gap — it is ground truth over its own seed
    /// accounting, not a scan of the projection. Cell-level provenance
    /// (`grid_cell_provenance`, [`value_provenance_projection`]) is unchanged
    /// by this — only this sheet-level summary's source moved.
    pub fn workbook_calc_projection(
        &self,
        last_recalc_tick: Option<u64>,
    ) -> Result<WorkbookCalcProjection, WorkbookSessionError> {
        let settings = self.context().workbook_calc_settings(self.workspace_id())?;
        let mut sheets = Vec::new();
        for row in self.sheets()? {
            let dirty = self
                .context()
                .has_undrained_edits(self.workspace_id(), row.node_id)?;
            sheets.push(SheetCalcSummaryProjection {
                grid_node_id: crate::workbook::sheet_grid_node_id(row.node_id),
                dirty,
            });
        }
        Ok(WorkbookCalcProjection {
            mode: calc_mode_projection(settings.calc_mode),
            last_recalc_tick,
            sheets,
        })
    }

    /// Set the workbook's calc mode (H5, §A.2: `set_workbook_calc_settings`,
    /// the `calc_mode` field only — date system/iteration settings are out of
    /// H5's scope, so they are read back and re-written unchanged).
    pub fn set_calc_mode(&mut self, mode: CalcModeProjection) -> Result<(), WorkbookSessionError> {
        let workspace_id = self.workspace_id().clone();
        let current = self.context().workbook_calc_settings(&workspace_id)?;
        self.context_mut().set_workbook_calc_settings(
            &workspace_id,
            WorkbookCalcSettings {
                calc_mode: calc_mode_from_projection(mode),
                ..current
            },
        )?;
        Ok(())
    }

    /// Explicitly recalculate the workbook (H5, §A.2: `recalculate_workbook`
    /// — Excel's F9). Drains every sheet carrying undrained authored edits
    /// under one shared tick; a session with nothing dirty is a genuine
    /// no-op (`WorkbookRecalcOutcome::drained_any()` is `false`, no tick is
    /// minted) — the caller uses this to decide whether to emit a
    /// `GridChanged` per drained sheet (H7 fans that out; H5's own dispatch
    /// only emits the `CalcStateChanged` delta).
    pub fn recalculate(&mut self) -> Result<WorkbookRecalcOutcome, WorkbookSessionError> {
        let workspace_id = self.workspace_id().clone();
        Ok(self.context_mut().recalculate_workbook(&workspace_id)?)
    }
}

/// Map a rejected calc-mode/recalc write to its typed [`dnacalc_skin_ir::IntentError`]
/// (§A.4). Neither `set_workbook_calc_settings` nor `recalculate_workbook` has
/// a calc-mode-specific typed rejection in the engine's own error set (a calc
/// mode change is a pure scheduling fact; §A.4 names no row for this family),
/// so every error here is the documented "unknown/unmapped" fallback — never
/// a panic, never silently dropped. Kept as its own function (rather than
/// inlining `format!` at each call site) so a future §A.4 row for this family
/// has exactly one place to add a real match arm.
#[must_use]
pub fn present_calc_rejection(error: &WorkbookSessionError) -> dnacalc_skin_ir::IntentError {
    dnacalc_skin_ir::IntentError::GenericEngineRejection {
        debug: format!("{error:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxfunc_core::value::CalcValue;

    /// H5 acceptance (1): a freshly created workbook (default Automatic mode)
    /// projects `mode = Automatic`, no recalc tick yet, and no sheet dirty.
    #[test]
    fn workbook_calc_projection_defaults_to_automatic_and_clean() {
        let mut session = WorkbookSession::create("workbook:h5-defaults").unwrap();
        let sheet = session.add_sheet("Sheet1").unwrap();
        session
            .set_grid_cell_value(sheet, 1, 1, CalcValue::number(7.0))
            .unwrap();

        let projection = session.workbook_calc_projection(None).unwrap();
        assert_eq!(projection.mode, CalcModeProjection::Automatic);
        assert_eq!(projection.last_recalc_tick, None);
        assert_eq!(projection.sheets.len(), 1);
        assert!(
            !projection.sheets[0].dirty,
            "Automatic mode publishes immediately, so nothing is dirty"
        );
    }

    /// H5 acceptance (2): Manual mode -> edit -> cell provenance `Stale{since}`
    /// and the value unchanged -> `Recalculate` -> `Calculated{tick}` and the
    /// value updated. This is host-core's own dispatch-boundary proof of the
    /// engine's `manual_mode_edit_leaves_stale_published_value_then_f9_refreshes`
    /// behavior (OxCalc `consumer.rs`), exercised through `WorkbookSession`.
    #[test]
    fn manual_mode_edit_stales_then_recalculate_refreshes() {
        let mut session = WorkbookSession::create("workbook:h5-manual").unwrap();
        let sheet = session.add_sheet("Sheet1").unwrap();
        session
            .set_grid_cell_value(sheet, 1, 1, CalcValue::number(7.0))
            .unwrap();
        session.enter_grid_cell(sheet, 1, 2, "=A1*3").unwrap();
        assert_eq!(
            session.grid_cell_value(sheet, 1, 2).unwrap(),
            Some(CalcValue::number(21.0)),
            "sanity: B1 = A1*3 = 21 under Automatic"
        );

        session.set_calc_mode(CalcModeProjection::Manual).unwrap();
        assert_eq!(
            session.workbook_calc_projection(None).unwrap().mode,
            CalcModeProjection::Manual
        );

        // Manual-mode edit: A1 = 10. Nothing recalculates.
        session.enter_grid_cell(sheet, 1, 1, "10").unwrap();

        // Value unchanged: A1 still reads the pre-edit 7.
        assert_eq!(
            session.grid_cell_value(sheet, 1, 1).unwrap(),
            Some(CalcValue::number(7.0)),
            "A1's published value stays the pre-edit 7 under Manual (no recalc ran)"
        );
        let a1_provenance = session.grid_cell_provenance(sheet, 1, 1).unwrap();
        assert!(
            matches!(a1_provenance, Some(ValueProvenanceProjection::Stale { .. })),
            "A1's published value is tagged Stale, not silently fresh, got {a1_provenance:?}"
        );
        // The per-sheet summary reflects the dirty sheet.
        let projection = session.workbook_calc_projection(None).unwrap();
        assert!(
            projection.sheets[0].dirty,
            "the sheet summary reports dirty while a Stale cell is published"
        );

        // Recalculate (F9): drains and publishes fresh Calculated values.
        let outcome = session.recalculate().unwrap();
        assert!(outcome.drained_any(), "F9 drained the dirty sheet");
        let tick = outcome.tick_id.expect("a drain mints a tick");

        assert_eq!(
            session.grid_cell_value(sheet, 1, 2).unwrap(),
            Some(CalcValue::number(30.0)),
            "after Recalculate, B1 = A1*3 = 30 (fresh value)"
        );
        assert_eq!(
            session.grid_cell_provenance(sheet, 1, 2).unwrap(),
            Some(ValueProvenanceProjection::Calculated { tick_id: tick }),
            "after Recalculate, B1's provenance is Calculated with the drain's tick"
        );

        let projection_after = session.workbook_calc_projection(Some(tick)).unwrap();
        assert!(
            !projection_after.sheets[0].dirty,
            "the sheet summary is clean once every published cell is fresh again"
        );
    }

    /// H5 follow-up (dtc-ajl.29): a brand-new cell under Manual mode — an
    /// address with no prior published entry — has nothing for the engine's
    /// own `mark_published_stale` to re-tag, so a Stale-scan over published
    /// cell provenance would false-negative here (zero `Stale` cells to
    /// find). The engine's `has_undrained_edits` readout has no such gap: it
    /// reads the seed set directly, so the sheet summary reports dirty even
    /// though `grid_cell_provenance` for the new cell is `None` (not `Stale`).
    #[test]
    fn brand_new_cell_under_manual_reports_dirty_with_zero_stale_cells() {
        let mut session = WorkbookSession::create("workbook:h5-brand-new-manual").unwrap();
        let sheet = session.add_sheet("Sheet1").unwrap();

        // Manual mode from the start: no cell has ever been published on this
        // sheet, so there is no prior `Calculated` entry for a later edit to
        // retag `Stale`.
        session.set_calc_mode(CalcModeProjection::Manual).unwrap();

        // A brand-new cell, never touched before.
        session.enter_grid_cell(sheet, 1, 1, "10").unwrap();

        // Zero Stale cells: the cell has no prior published entry at all, so
        // provenance reads `None`, not `Stale`.
        let provenance = session.grid_cell_provenance(sheet, 1, 1).unwrap();
        assert_eq!(
            provenance, None,
            "a brand-new cell under Manual has no prior published entry to retag Stale"
        );

        // Yet the sheet summary is honestly dirty: the engine's own
        // undrained-edits accounting sees the seed, independent of the
        // (false-negative-prone) Stale scan.
        let projection = session.workbook_calc_projection(None).unwrap();
        assert!(
            projection.sheets[0].dirty,
            "has_undrained_edits reports dirty for the brand-new cell even with zero Stale cells"
        );

        // Recalculate (F9) drains the seed; the sheet summary flips clean.
        let outcome = session.recalculate().unwrap();
        assert!(
            outcome.drained_any(),
            "F9 drained the brand-new cell's seed"
        );
        let tick = outcome.tick_id.expect("a drain mints a tick");
        let projection_after = session.workbook_calc_projection(Some(tick)).unwrap();
        assert!(
            !projection_after.sheets[0].dirty,
            "the sheet summary is clean once the engine's seed set is drained"
        );
    }

    /// H5 acceptance (3): `Recalculate` with nothing dirty (a fresh Automatic
    /// workbook, or a repeat call) is a genuine no-op — `drained_any()` is
    /// `false` and no tick is minted.
    #[test]
    fn recalculate_with_nothing_dirty_is_a_noop() {
        let mut session = WorkbookSession::create("workbook:h5-noop").unwrap();
        let sheet = session.add_sheet("Sheet1").unwrap();
        session
            .set_grid_cell_value(sheet, 1, 1, CalcValue::number(7.0))
            .unwrap();

        // Automatic mode already recalculated on write, so nothing is dirty.
        let outcome = session.recalculate().unwrap();
        assert!(
            !outcome.drained_any(),
            "Recalculate under Automatic with nothing dirty is a no-op"
        );
        assert_eq!(outcome.tick_id, None, "a no-op mints no tick");

        // A second consecutive call after a genuine drain also finds nothing
        // left to drain.
        session.set_calc_mode(CalcModeProjection::Manual).unwrap();
        session.enter_grid_cell(sheet, 1, 1, "11").unwrap();
        let first = session.recalculate().unwrap();
        assert!(
            first.drained_any(),
            "the first F9 after a Manual edit drains"
        );
        let second = session.recalculate().unwrap();
        assert!(
            !second.drained_any(),
            "a repeat Recalculate with nothing newly dirty is a no-op"
        );
        assert_eq!(second.tick_id, None);
    }
}

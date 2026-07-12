//! The Strip — SHELL_SPEC.md §7 readouts and their truth sources.
//!
//! Slot law: a slot is **wired** to a `WorkspaceState` projection where one
//! exists, shows **honest absence** (an em-dash + reason, never a fake OK)
//! where the projection is not present, and the reserved slots render
//! **NOTHING** — feed health until G7, and the parity slot for the whole
//! phase (PARITY_TRUST_UX.md §5/§8: an absent feature renders as nothing,
//! not as "no evidence").

use dnacalc_skin_ir::state::{SharedSkinState, WorkspaceRecalcMode};
use dnacalc_skin_ir::workspace::{CalcModeProjection, WorkspaceState};

/// The strip slots, left to right (SHELL_SPEC §1 strip contents plus the
/// reserved parity slot, rightmost group).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StripSlot {
    Calc,
    Dirty,
    LastRun,
    Feeds,
    Locale,
    Revision,
    Zoom,
    AgentLight,
    Parity,
}

impl StripSlot {
    /// Display order.
    pub const ALL: [StripSlot; 9] = [
        StripSlot::Calc,
        StripSlot::Dirty,
        StripSlot::LastRun,
        StripSlot::Feeds,
        StripSlot::Locale,
        StripSlot::Revision,
        StripSlot::Zoom,
        StripSlot::AgentLight,
        StripSlot::Parity,
    ];

    /// Stable id used as the slot's `data-slot` attribute.
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Calc => "calc",
            Self::Dirty => "dirty",
            Self::LastRun => "last-run",
            Self::Feeds => "feeds",
            Self::Locale => "locale",
            Self::Revision => "revision",
            Self::Zoom => "zoom",
            Self::AgentLight => "agent-light",
            Self::Parity => "parity",
        }
    }
}

/// What a strip slot renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StripSlotContent {
    /// Wired to a live projection.
    Text(String),
    /// The projection is not present — honest absence: an em-dash with the
    /// reason as its accessible title. Never a fake OK.
    Absent(&'static str),
    /// Reserved: the slot exists in layout math and renders NOTHING —
    /// no text, no glyph, no dash (PARITY_TRUST_UX §8).
    Nothing,
}

/// Resolve one slot's content from the projections. Pure — the DOM strip
/// maps this 1:1, so the render-nothing law is testable without a browser.
#[must_use]
pub fn strip_slot_content(
    slot: StripSlot,
    workspace: &WorkspaceState,
    shared: &SharedSkinState,
) -> StripSlotContent {
    match slot {
        StripSlot::Calc => match &workspace.workbook_calc {
            Some(calc) => StripSlotContent::Text(
                match calc.mode {
                    CalcModeProjection::Automatic => "calc: auto",
                    CalcModeProjection::Manual => "calc: manual",
                }
                .to_string(),
            ),
            // A tree session without workbook calc settings still has the
            // shared recalc mode — honest, host-owned truth.
            None => StripSlotContent::Text(format!(
                "calc: {}",
                match shared.recalc_mode {
                    WorkspaceRecalcMode::Auto => "auto",
                    WorkspaceRecalcMode::Manual => "manual",
                }
            )),
        },
        StripSlot::Dirty => match &workspace.workbook_calc {
            Some(calc) => {
                let dirty = calc.sheets.iter().filter(|sheet| sheet.dirty).count();
                StripSlotContent::Text(format!("dirty: {dirty}"))
            }
            None => StripSlotContent::Absent("no workbook calc projection"),
        },
        StripSlot::LastRun => match &workspace.last_run {
            Some(run) => {
                let total_micros: u128 = run.phase_timings_micros.values().sum();
                StripSlotContent::Text(format!("run: {:.1} ms", total_micros as f64 / 1000.0))
            }
            None => StripSlotContent::Absent("no calc run yet"),
        },
        // Feed health is G7: absent until then — the slot renders nothing
        // rather than a fake OK (SHELL_SPEC §7).
        StripSlot::Feeds => StripSlotContent::Nothing,
        // Locale is a host capability, not a WorkspaceState projection.
        StripSlot::Locale => StripSlotContent::Absent("host locale capability not projected"),
        StripSlot::Revision => match &workspace.revision_history.current_revision_id {
            Some(id) => StripSlotContent::Text(format!("rev: {}", short_revision(id))),
            None => StripSlotContent::Absent("no revision history projected"),
        },
        // Zoom is stage-owned; no stage publishes zoom in S0.
        StripSlot::Zoom => StripSlotContent::Absent("stage zoom not published"),
        // Agent light (mechanism 20) needs dispatcher origin attribution,
        // which is not projected yet.
        StripSlot::AgentLight => {
            StripSlotContent::Absent("intent origin attribution not projected")
        }
        // Parity: reserved for the evidence story's return; renders nothing
        // this phase — a placeholder parity mark is a bug
        // (PARITY_TRUST_UX §5/§8).
        StripSlot::Parity => StripSlotContent::Nothing,
    }
}

fn short_revision(id: &str) -> &str {
    &id[..id.len().min(8)]
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use dnacalc_skin_ir::workspace::{
        CalcRunProjection, CalcRunStateProjection, PhaseKeyProjection, SheetCalcSummaryProjection,
        WorkbookCalcProjection,
    };

    use super::*;

    fn empty() -> (WorkspaceState, SharedSkinState) {
        (WorkspaceState::default(), SharedSkinState::default())
    }

    /// PARITY_TRUST_UX §8 + SHELL_SPEC §7: the reserved slots render
    /// NOTHING — and nothing else does, so "renders nothing" stays an
    /// exclusive, deliberate state.
    #[test]
    fn parity_and_feeds_render_nothing_and_only_they_do() {
        let (workspace, shared) = empty();
        for slot in StripSlot::ALL {
            let content = strip_slot_content(slot, &workspace, &shared);
            match slot {
                StripSlot::Parity | StripSlot::Feeds => {
                    assert_eq!(
                        content,
                        StripSlotContent::Nothing,
                        "{slot:?} is reserved and must render nothing"
                    );
                }
                _ => {
                    assert_ne!(
                        content,
                        StripSlotContent::Nothing,
                        "{slot:?} must render truth or honest absence, never nothing"
                    );
                }
            }
        }
    }

    #[test]
    fn parity_renders_nothing_even_with_full_projections() {
        // The reserved slot must not light up because data exists elsewhere.
        let (mut workspace, shared) = empty();
        workspace.workbook_calc = Some(WorkbookCalcProjection::default());
        workspace.revision_history.current_revision_id = Some("abcdef0123456789".into());
        assert_eq!(
            strip_slot_content(StripSlot::Parity, &workspace, &shared),
            StripSlotContent::Nothing
        );
    }

    #[test]
    fn calc_and_dirty_wire_to_workbook_calc_when_present() {
        let (mut workspace, shared) = empty();
        workspace.workbook_calc = Some(WorkbookCalcProjection {
            mode: CalcModeProjection::Manual,
            last_recalc_tick: Some(7),
            sheets: vec![
                SheetCalcSummaryProjection {
                    grid_node_id: dnacalc_skin_ir::identity::NodeId::new("a"),
                    dirty: true,
                },
                SheetCalcSummaryProjection {
                    grid_node_id: dnacalc_skin_ir::identity::NodeId::new("b"),
                    dirty: false,
                },
            ],
        });
        assert_eq!(
            strip_slot_content(StripSlot::Calc, &workspace, &shared),
            StripSlotContent::Text("calc: manual".into())
        );
        assert_eq!(
            strip_slot_content(StripSlot::Dirty, &workspace, &shared),
            StripSlotContent::Text("dirty: 1".into())
        );
    }

    #[test]
    fn calc_falls_back_to_shared_recalc_mode_for_tree_sessions() {
        let (workspace, mut shared) = empty();
        shared.recalc_mode = WorkspaceRecalcMode::Manual;
        assert_eq!(
            strip_slot_content(StripSlot::Calc, &workspace, &shared),
            StripSlotContent::Text("calc: manual".into())
        );
    }

    #[test]
    fn last_run_and_revision_wire_or_show_honest_absence() {
        let (mut workspace, shared) = empty();
        assert_eq!(
            strip_slot_content(StripSlot::LastRun, &workspace, &shared),
            StripSlotContent::Absent("no calc run yet")
        );
        let mut timings = BTreeMap::new();
        timings.insert(PhaseKeyProjection::RuntimeSetup, 12_345u128);
        workspace.last_run = Some(CalcRunProjection {
            run_state: CalcRunStateProjection::Published,
            evaluation_order: Vec::new(),
            runtime_effect_count: 0,
            runtime_effects: Vec::new(),
            runtime_overlay_count: 0,
            runtime_overlays: Vec::new(),
            derivation_trace_count: 0,
            derivation_traces: Vec::new(),
            invalidated_nodes: Vec::new(),
            binding_diagnostics: Vec::new(),
            phase_timings_micros: timings,
            diagnostics: Vec::new(),
        });
        assert_eq!(
            strip_slot_content(StripSlot::LastRun, &workspace, &shared),
            StripSlotContent::Text("run: 12.3 ms".into())
        );

        workspace.revision_history.current_revision_id = Some("abcdef0123456789".into());
        assert_eq!(
            strip_slot_content(StripSlot::Revision, &workspace, &shared),
            StripSlotContent::Text("rev: abcdef01".into())
        );
    }

    #[test]
    fn unprojected_slots_are_honest_absence_not_fake_ok() {
        let (workspace, shared) = empty();
        for slot in [StripSlot::Locale, StripSlot::Zoom, StripSlot::AgentLight] {
            assert!(matches!(
                strip_slot_content(slot, &workspace, &shared),
                StripSlotContent::Absent(_)
            ));
        }
    }
}

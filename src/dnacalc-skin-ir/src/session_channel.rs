//! The worker-session boundary (B.2.2–B.2.4): a transport-agnostic protocol
//! between the main-thread proxy and the session executor that owns the
//! OxCalc context.
//!
//! The spike (`CORE_ENGINE_HOST_WORKER_PASSIVITY_SPIKE.md`) established that
//! the session is a pure function of owned input, so it can run off the UI
//! thread — a wasm web worker, or a native thread. The synchronous
//! [`Dispatcher`](crate::Dispatcher) and [`PreviewService`](crate::PreviewService)
//! contracts the skins depend on cannot cross an async boundary, so the host
//! splits into a **main-thread proxy** (owns the Leptos signals, returns a
//! `Pending` receipt immediately) and a **session executor** (owns the engine,
//! produces a [`SessionResponse`]). This module is the pure, transport-agnostic
//! contract between them — no Leptos, no web-sys — so the whole boundary is
//! unit-testable in-process. The host wires the executor and the transports
//! (an in-process one for tests/native, a `postMessage` one for the worker).

use serde::{Deserialize, Serialize};

use crate::intent::{IntentReceipt, WorkspaceDelta, WorkspaceDeltaChange, WorkspaceIntent};
use crate::selection::SelectionState;
use crate::workspace::WorkspaceState;

/// A sequence-stamped intent submitted to the session executor.
///
/// The proxy assigns `seq` monotonically and the executor echoes it in the
/// [`SessionResponse`], so the proxy can discard a superseded run's late
/// arrival and detect an out-of-order or dropped response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentEnvelope {
    pub seq: u64,
    pub intent: WorkspaceIntent,
}

impl IntentEnvelope {
    #[must_use]
    pub fn new(seq: u64, intent: WorkspaceIntent) -> Self {
        Self { seq, intent }
    }
}

/// What the executor returns for an envelope.
///
/// `snapshot` is `Some` exactly when the proxy must reset its mirror — the
/// receipt's delta carries a change the mirror cannot apply (see
/// [`is_delta_applicable`]) — and then holds the authoritative full state.
/// When the delta is fully applicable, `snapshot` is `None` and only the
/// delta crosses the boundary.
///
/// `selection` is `Some` exactly when the intent changed the host selection —
/// the MIXED structural intents (`AddNode`/`RenameNode`/`MoveNode`/`DeleteNode`
/// select the affected node, which only the executor knows). Selection is
/// otherwise main-thread view-state the proxy owns, so a `None` here means
/// "leave the selection alone."
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionResponse {
    pub seq: u64,
    pub receipt: IntentReceipt,
    pub snapshot: Option<WorkspaceState>,
    pub selection: Option<SelectionState>,
}

impl SessionResponse {
    /// Build the response an executor should send for `receipt`: the receipt
    /// plus a snapshot whenever the receipt's delta is not fully applicable to
    /// a retained mirror, and the resulting selection when the intent changed
    /// it. This is the executor's side of the delta-vs-snapshot decision; the
    /// proxy never has to guess.
    #[must_use]
    pub fn for_receipt(
        seq: u64,
        receipt: IntentReceipt,
        state: &WorkspaceState,
        selection: Option<SelectionState>,
    ) -> Self {
        let snapshot = (!delta_is_fully_applicable(&receipt.delta)).then(|| state.clone());
        Self {
            seq,
            receipt,
            snapshot,
            selection,
        }
    }
}

/// A recalculation in flight (B.2.2 cancellation story).
///
/// `token` versions runs so a superseded run's late arrival is discarded;
/// `started_value_epoch` records the input state the run began from, so a
/// result computed against an input the user has since changed can be dropped
/// rather than published over fresher state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingRun {
    pub token: u64,
    pub started_value_epoch: u64,
}

/// One intent's dispatch→apply timing, emitted to the telemetry sink so the
/// 60fps goal is falsifiable (B.2.4 `frame-telemetry-hooks`). The proxy fills
/// these as it applies each response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameMetric {
    pub intent_seq: u64,
    pub dispatch_to_apply_micros: u64,
    /// Deferred edits this intent coalesced away before it ran.
    pub coalesced: usize,
    /// Whether applying the response required a full-snapshot resync.
    pub resynced: bool,
}

/// Why the proxy must request a fresh snapshot instead of applying a delta.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResyncReason {
    #[error("projection_seq gap: mirror at {expected}, delta starts at {got}")]
    SequenceGap { expected: u64, got: u64 },
    #[error("delta carries a change the mirror cannot apply: {kind}")]
    UnrepresentableChange { kind: &'static str },
}

/// A stable id for a delta change kind — for resync reasons, the coverage
/// audit, and telemetry.
#[must_use]
pub fn change_kind(change: &WorkspaceDeltaChange) -> &'static str {
    use WorkspaceDeltaChange as C;
    match change {
        C::FullReset => "full_reset",
        C::Structural(_) => "structural",
        C::NodesChanged(_) => "nodes_changed",
        C::ValuesChanged(_) => "values_changed",
        C::DepsChanged(_) => "deps_changed",
        C::CalcRun(_) => "calc_run",
        C::ClipboardChanged(_) => "clipboard_changed",
        C::FormulaReferenceInserted(_) => "formula_reference_inserted",
        C::CandidateChanged(_) => "candidate_changed",
        C::CandidateRemoved(_) => "candidate_removed",
        C::ScenarioChanged(_) => "scenario_changed",
        C::ScenarioRemoved(_) => "scenario_removed",
        C::SweepChanged(_) => "sweep_changed",
        C::SweepRemoved(_) => "sweep_removed",
        C::GridChanged(_) => "grid_changed",
        C::GridOverlaysChanged { .. } => "grid_overlays_changed",
        C::GridAuthoredChanged { .. } => "grid_authored_changed",
        C::GridCellEntered { .. } => "grid_cell_entered",
        C::DefinedNamesChanged(_) => "defined_names_changed",
        C::CalcStateChanged(_) => "calc_state_changed",
        C::SheetsChanged(_) => "sheets_changed",
    }
}

/// Whether a single delta change is a complete-replacement patch the mirror
/// can apply without the full snapshot.
///
/// Conservative by construction: only a change that carries its *entire* new
/// projection value qualifies. Key-only hints (`Structural`, `NodesChanged`),
/// partial patches (`ValuesChanged` omits the node's calc-state/value-epoch,
/// `DepsChanged` is a dirty-set hint), the speculation collections (whose
/// upsert semantics are the host's, not reconstructable from the change
/// alone), and `FullReset` do not — the executor ships a snapshot for those.
///
/// This is the delta-coverage audit's authority: `delta_coverage_is_total`
/// asserts every variant is classified, so adding a `WorkspaceDeltaChange`
/// forces a coverage decision here rather than silently defaulting to a
/// possibly-wrong patch.
#[must_use]
pub fn is_delta_applicable(change: &WorkspaceDeltaChange) -> bool {
    use WorkspaceDeltaChange as C;
    match change {
        // Complete replacements of a whole projection field.
        // GridChanged carries the entire new windowed grid projection for one
        // node, so the mirror replaces it in place (the whole point of "viewing
        // is subscribing" -- a grid recompute must not force a full snapshot).
        // GridOverlaysChanged carries the entire new overlay bundle for one node,
        // so the mirror replaces that field in place without a snapshot.
        // GridAuthoredChanged carries the entire new windowed authored layer for
        // one node (H3), so the mirror replaces each cell's `authored` field in
        // place the same way, without a snapshot.
        // DefinedNamesChanged carries the entire new defined-name catalog (H4),
        // so the mirror replaces `WorkspaceState::defined_names` in place.
        // CalcStateChanged carries the entire new calc-mode/recalc projection
        // (H5), so the mirror replaces `WorkspaceState::workbook_calc` in place.
        // SheetsChanged carries the entire new sheet list (Phase 1 Part A), so
        // the mirror replaces `WorkspaceState::sheets` in place.
        C::CalcRun(_)
        | C::ClipboardChanged(_)
        | C::GridChanged(_)
        | C::GridOverlaysChanged { .. }
        | C::GridAuthoredChanged { .. }
        | C::DefinedNamesChanged(_)
        | C::CalcStateChanged(_)
        | C::SheetsChanged(_) => true,
        // A UI hint only — there is no projection-state change to mirror.
        // GridCellEntered is the H6 entry-verb receipt payload: the
        // GridChanged(s) host-core emits in the same delta (dtc-j7n8.18) —
        // the edited sheet's, plus one per other sheet the edit's cross-sheet
        // recalc moved — are what actually patch the mirror.
        C::FormulaReferenceInserted(_) | C::GridCellEntered { .. } => true,
        // Key-only, partial, collection-upsert, or full-reset: the snapshot is
        // authoritative and the proxy resyncs.
        C::FullReset
        | C::Structural(_)
        | C::NodesChanged(_)
        | C::ValuesChanged(_)
        | C::DepsChanged(_)
        | C::CandidateChanged(_)
        | C::CandidateRemoved(_)
        | C::ScenarioChanged(_)
        | C::ScenarioRemoved(_)
        | C::SweepChanged(_)
        | C::SweepRemoved(_) => false,
    }
}

/// Whether every change in a delta is mirror-applicable, so the executor can
/// send delta-only (no snapshot).
#[must_use]
pub fn delta_is_fully_applicable(delta: &WorkspaceDelta) -> bool {
    delta.changes.iter().all(is_delta_applicable)
}

/// Apply a delta onto a retained mirror, or report why a resync is needed.
///
/// All-or-nothing: if the sequence does not line up or any change is
/// unrepresentable, the mirror is left **untouched** and the caller requests a
/// fresh snapshot — a partial patch never leaves the mirror inconsistent.
pub fn apply_delta(state: &mut WorkspaceState, delta: &WorkspaceDelta) -> Result<(), ResyncReason> {
    if delta.from_seq != state.projection_seq {
        return Err(ResyncReason::SequenceGap {
            expected: state.projection_seq,
            got: delta.from_seq,
        });
    }
    if let Some(change) = delta
        .changes
        .iter()
        .find(|change| !is_delta_applicable(change))
    {
        return Err(ResyncReason::UnrepresentableChange {
            kind: change_kind(change),
        });
    }
    for change in &delta.changes {
        match change {
            WorkspaceDeltaChange::CalcRun(run) => state.last_run = Some(run.clone()),
            WorkspaceDeltaChange::ClipboardChanged(clipboard) => {
                state.clipboard = clipboard.clone();
            }
            WorkspaceDeltaChange::GridChanged(grid) => {
                state.grids.insert(grid.grid_node_id.clone(), grid.clone());
            }
            WorkspaceDeltaChange::GridOverlaysChanged {
                grid_node_id,
                overlays,
                overlay_epoch,
            } => {
                // Patch overlays in place; the cells and projection epoch are
                // untouched (an overlay-only tick). If the grid is not yet
                // mirrored, skip - a grid first appears via a GridChanged (which
                // carries overlays), so an overlay-only patch for an unknown grid
                // does not occur in the normal flow.
                if let Some(grid) = state.grids.get_mut(grid_node_id) {
                    grid.overlays = overlays.clone();
                    grid.overlay_epoch = *overlay_epoch;
                }
            }
            WorkspaceDeltaChange::GridAuthoredChanged {
                grid_node_id,
                cells,
                authored_epoch,
            } => {
                // Patch each windowed cell's `authored` field in place by
                // (row, col); the computed value and its epoch are untouched
                // (an authored-only tick). If the grid is not yet mirrored,
                // skip - a grid first appears via a GridChanged, so an
                // authored-only patch for an unknown grid does not occur in
                // the normal flow (mirrors the GridOverlaysChanged handling
                // above).
                if let Some(grid) = state.grids.get_mut(grid_node_id) {
                    for authored in cells {
                        if let Some(cell) = grid
                            .cells
                            .iter_mut()
                            .find(|cell| cell.row == authored.row && cell.col == authored.col)
                        {
                            cell.authored = Some(authored.clone());
                        }
                    }
                    grid.authored_epoch = *authored_epoch;
                }
            }
            WorkspaceDeltaChange::DefinedNamesChanged(defined_names) => {
                state.defined_names = defined_names.clone();
            }
            WorkspaceDeltaChange::CalcStateChanged(workbook_calc) => {
                state.workbook_calc = Some(workbook_calc.clone());
            }
            WorkspaceDeltaChange::SheetsChanged(sheets) => {
                state.sheets = sheets.clone();
            }
            // FormulaReferenceInserted is a UI hint with no projection state.
            // Everything else was rejected by the applicability guard above.
            _ => {}
        }
    }
    state.projection_seq = delta.to_seq;
    Ok(())
}

/// A FIFO of pending intents with per-node coalescing of deferred content
/// edits (B.2.4 `backpressure-coalescing`).
///
/// While a run is in flight the proxy parks incoming intents here; a newer
/// `EditContentDeferred` for a node supersedes the one already queued for it,
/// so a burst of keystrokes collapses to the latest value instead of running
/// a recalc per character. Non-deferred intents and edits to *other* nodes
/// keep their order.
#[derive(Debug, Default)]
pub struct PendingIntentQueue {
    items: Vec<(u64, WorkspaceIntent)>,
    coalesced: usize,
}

impl PendingIntentQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue `intent` under `seq`, coalescing a deferred content edit onto
    /// an existing queued edit for the same node. Returns the superseded seq
    /// when it coalesced (so the caller can record a `Coalesced` outcome).
    pub fn enqueue(&mut self, seq: u64, intent: WorkspaceIntent) -> Option<u64> {
        if let WorkspaceIntent::EditContentDeferred { node, .. } = &intent
            && let Some(slot) = self.items.iter_mut().find(|(_, queued)| {
                matches!(
                    queued,
                    WorkspaceIntent::EditContentDeferred { node: queued_node, .. }
                        if queued_node == node
                )
            })
        {
            let superseded = slot.0;
            *slot = (seq, intent);
            self.coalesced += 1;
            return Some(superseded);
        }
        self.items.push((seq, intent));
        None
    }

    /// Take everything queued, leaving the queue empty (the coalesced tally is
    /// retained for telemetry until [`Self::reset`]).
    pub fn drain(&mut self) -> Vec<(u64, WorkspaceIntent)> {
        std::mem::take(&mut self.items)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// How many deferred edits have been coalesced away since the last reset.
    #[must_use]
    pub fn coalesced_count(&self) -> usize {
        self.coalesced
    }

    /// Clear the coalesced tally (call after emitting a [`FrameMetric`]).
    pub fn reset_coalesced(&mut self) {
        self.coalesced = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{NodeId, NodeKey};
    use crate::intent::StructuralDeltaProjection;
    use crate::workspace::{
        CalcRunProjection, CalcRunStateProjection, ClipboardOperationProjection,
        ClipboardPayloadProjection, ClipboardProjection, DefinedNameProjection,
        DefinedNameScopeProjection, DefinedNameTargetProjection, DefinedNamesProjection,
        GridAuthoredCellProjection, GridAuthoredKindProjection, GridCellProjection,
        GridEditabilityProjection, GridMergedOverlayDescriptor, GridOverlayBundle, GridOverlayRect,
        GridProjection, GridRectProjection, NodeValueProjection, SheetProjection,
    };

    fn sample_calc_run() -> CalcRunProjection {
        CalcRunProjection {
            run_state: CalcRunStateProjection::VerifiedClean,
            evaluation_order: Vec::new(),
            runtime_effect_count: 0,
            runtime_effects: Vec::new(),
            runtime_overlay_count: 0,
            runtime_overlays: Vec::new(),
            derivation_trace_count: 0,
            derivation_traces: Vec::new(),
            invalidated_nodes: Vec::new(),
            binding_diagnostics: Vec::new(),
            phase_timings_micros: std::collections::BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }

    fn sample_clipboard() -> ClipboardProjection {
        ClipboardProjection {
            operation: ClipboardOperationProjection::Copy,
            payload: ClipboardPayloadProjection::Values { nodes: Vec::new() },
            plain_text: None,
        }
    }

    fn sample_grid_projection() -> GridProjection {
        GridProjection {
            grid_node_key: NodeKey::new("sheet"),
            grid_node_id: NodeId::new("Sheet1"),
            grid_id: "book:grid:sheet:grid".to_string(),
            max_rows: 1_048_576,
            max_cols: 16_384,
            cells: vec![GridCellProjection {
                row: 1,
                col: 1,
                value: NodeValueProjection::Number {
                    raw: "7".to_string(),
                    display: "7".to_string(),
                },
                value_epoch: 1,
                authored: None,
                provenance: None,
            }],
            projection_epoch: 1,
            overlays: GridOverlayBundle::default(),
            overlay_epoch: 0,
            differential_clean: true,
            authored_epoch: 0,
        }
    }

    fn mirror_at(seq: u64) -> WorkspaceState {
        WorkspaceState {
            projection_seq: seq,
            ..WorkspaceState::default()
        }
    }

    fn delta(from: u64, to: u64, changes: Vec<WorkspaceDeltaChange>) -> WorkspaceDelta {
        WorkspaceDelta {
            from_seq: from,
            to_seq: to,
            changes,
        }
    }

    #[test]
    fn delta_coverage_is_total_and_split_as_documented() {
        // Every variant is classified (this match is exhaustive, so a new
        // variant fails to compile until it is given a coverage decision).
        let applicable = [
            WorkspaceDeltaChange::CalcRun(sample_calc_run()),
            WorkspaceDeltaChange::ClipboardChanged(None),
            WorkspaceDeltaChange::GridChanged(sample_grid_projection()),
            WorkspaceDeltaChange::GridOverlaysChanged {
                grid_node_id: NodeId::new("Sheet1"),
                overlays: GridOverlayBundle::default(),
                overlay_epoch: 1,
            },
            WorkspaceDeltaChange::GridAuthoredChanged {
                grid_node_id: NodeId::new("Sheet1"),
                cells: vec![GridAuthoredCellProjection {
                    row: 1,
                    col: 1,
                    kind: GridAuthoredKindProjection::Literal,
                    literal_text: Some("7".to_string()),
                    source_text: None,
                    editability: GridEditabilityProjection::Editable,
                }],
                authored_epoch: 1,
            },
            WorkspaceDeltaChange::GridCellEntered {
                grid_node_id: NodeId::new("Sheet1"),
                row: 1,
                col: 1,
                outcome: crate::workspace::GridEntryOutcomeProjection::Cleared,
            },
            WorkspaceDeltaChange::DefinedNamesChanged(DefinedNamesProjection {
                entries: vec![DefinedNameProjection {
                    scope: DefinedNameScopeProjection::Workbook,
                    name: "Rate".to_string(),
                    target: DefinedNameTargetProjection::Static(GridRectProjection {
                        top_row: 1,
                        left_col: 1,
                        bottom_row: 1,
                        right_col: 1,
                    }),
                    is_dynamic: false,
                }],
            }),
            WorkspaceDeltaChange::CalcStateChanged(crate::workspace::WorkbookCalcProjection {
                mode: crate::workspace::CalcModeProjection::Manual,
                last_recalc_tick: Some(1),
                sheets: vec![crate::workspace::SheetCalcSummaryProjection {
                    grid_node_id: NodeId::new("Sheet1"),
                    dirty: false,
                }],
            }),
            WorkspaceDeltaChange::SheetsChanged(vec![SheetProjection {
                grid_node_id: NodeId::new("sheet:1"),
                display_name: "Sheet1".to_string(),
                position: 0,
            }]),
        ];
        for change in &applicable {
            assert!(
                is_delta_applicable(change),
                "{} should apply",
                change_kind(change)
            );
        }
        let resync = [
            WorkspaceDeltaChange::FullReset,
            WorkspaceDeltaChange::Structural(StructuralDeltaProjection::default()),
            WorkspaceDeltaChange::NodesChanged(vec![NodeKey::new("k")]),
            WorkspaceDeltaChange::ValuesChanged(Vec::new()),
            WorkspaceDeltaChange::DepsChanged(Vec::new()),
            WorkspaceDeltaChange::CandidateRemoved("h".to_string()),
            WorkspaceDeltaChange::ScenarioRemoved("s".to_string()),
            WorkspaceDeltaChange::SweepRemoved("w".to_string()),
        ];
        for change in &resync {
            assert!(
                !is_delta_applicable(change),
                "{} should resync",
                change_kind(change)
            );
        }
    }

    #[test]
    fn apply_delta_patches_complete_replacements_and_advances_seq() {
        let mut mirror = mirror_at(4);
        let run = sample_calc_run();
        let clipboard = Some(sample_clipboard());
        let result = apply_delta(
            &mut mirror,
            &delta(
                4,
                5,
                vec![
                    WorkspaceDeltaChange::CalcRun(run.clone()),
                    WorkspaceDeltaChange::ClipboardChanged(clipboard.clone()),
                ],
            ),
        );
        assert!(result.is_ok());
        assert_eq!(mirror.projection_seq, 5);
        assert_eq!(mirror.last_run, Some(run));
        assert_eq!(mirror.clipboard, clipboard);
    }

    #[test]
    fn apply_delta_patches_grid_changed_in_place() {
        // "Viewing is subscribing": a grid recompute streams as a GridChanged
        // delta the mirror applies in place (no full snapshot/resync), landing
        // the windowed projection under its sheet node and advancing the seq.
        let mut mirror = mirror_at(7);
        let grid = sample_grid_projection();
        let result = apply_delta(
            &mut mirror,
            &delta(7, 8, vec![WorkspaceDeltaChange::GridChanged(grid.clone())]),
        );
        assert!(result.is_ok());
        assert_eq!(mirror.projection_seq, 8);
        assert_eq!(mirror.grids.get(&grid.grid_node_id), Some(&grid));
    }

    #[test]
    fn apply_delta_patches_grid_overlays_changed_in_place() {
        // An overlay-only tick streams as GridOverlaysChanged: the mirror patches
        // overlays/overlay_epoch in place without disturbing the cells, and never
        // forces a resync.
        let mut mirror = mirror_at(7);
        let grid = sample_grid_projection();
        apply_delta(
            &mut mirror,
            &delta(7, 8, vec![WorkspaceDeltaChange::GridChanged(grid.clone())]),
        )
        .unwrap();
        let cells_before = mirror.grids.get(&grid.grid_node_id).unwrap().cells.clone();

        let overlays = GridOverlayBundle {
            merged: vec![GridMergedOverlayDescriptor {
                rect: GridOverlayRect {
                    top_row: 1,
                    left_col: 1,
                    bottom_row: 2,
                    right_col: 2,
                    clipped_top: false,
                    clipped_left: false,
                    clipped_bottom: false,
                    clipped_right: false,
                },
            }],
            ..Default::default()
        };
        let result = apply_delta(
            &mut mirror,
            &delta(
                8,
                9,
                vec![WorkspaceDeltaChange::GridOverlaysChanged {
                    grid_node_id: grid.grid_node_id.clone(),
                    overlays: overlays.clone(),
                    overlay_epoch: 3,
                }],
            ),
        );
        assert!(result.is_ok());
        assert_eq!(mirror.projection_seq, 9);
        let mirrored = mirror.grids.get(&grid.grid_node_id).unwrap();
        assert_eq!(mirrored.overlays, overlays);
        assert_eq!(mirrored.overlay_epoch, 3);
        assert_eq!(
            mirrored.cells, cells_before,
            "an overlay-only patch must leave the cells untouched"
        );
    }

    #[test]
    fn apply_delta_patches_grid_authored_changed_in_place() {
        // An authored-only tick (an edit landed, or the interest window moved)
        // streams as GridAuthoredChanged: the mirror patches each windowed
        // cell's `authored` field and `authored_epoch` in place, without
        // disturbing the computed values, and never forces a resync (H3).
        let mut mirror = mirror_at(7);
        let grid = sample_grid_projection();
        apply_delta(
            &mut mirror,
            &delta(7, 8, vec![WorkspaceDeltaChange::GridChanged(grid.clone())]),
        )
        .unwrap();
        let values_before: Vec<_> = mirror
            .grids
            .get(&grid.grid_node_id)
            .unwrap()
            .cells
            .iter()
            .map(|cell| (cell.row, cell.col, cell.value.clone(), cell.value_epoch))
            .collect();

        let authored_cell = GridAuthoredCellProjection {
            row: 1,
            col: 1,
            kind: GridAuthoredKindProjection::Formula,
            literal_text: None,
            source_text: Some("=A1*3".to_string()),
            editability: GridEditabilityProjection::Editable,
        };
        let result = apply_delta(
            &mut mirror,
            &delta(
                8,
                9,
                vec![WorkspaceDeltaChange::GridAuthoredChanged {
                    grid_node_id: grid.grid_node_id.clone(),
                    cells: vec![authored_cell.clone()],
                    authored_epoch: 2,
                }],
            ),
        );
        assert!(result.is_ok());
        assert_eq!(mirror.projection_seq, 9);
        let mirrored = mirror.grids.get(&grid.grid_node_id).unwrap();
        assert_eq!(mirrored.authored_epoch, 2);
        assert_eq!(
            mirrored.cells[0].authored,
            Some(authored_cell),
            "the matching cell's authored field is patched in place"
        );
        let values_after: Vec<_> = mirrored
            .cells
            .iter()
            .map(|cell| (cell.row, cell.col, cell.value.clone(), cell.value_epoch))
            .collect();
        assert_eq!(
            values_after, values_before,
            "an authored-only patch must leave the computed values untouched"
        );
    }

    #[test]
    fn apply_delta_patches_defined_names_changed_in_place() {
        // DefinedNamesChanged carries the entire new catalog (H4): the mirror
        // replaces `WorkspaceState::defined_names` in place, without a
        // snapshot, exactly like the other complete-replacement deltas.
        let mut mirror = mirror_at(3);
        assert_eq!(mirror.defined_names, DefinedNamesProjection::default());

        let catalog = DefinedNamesProjection {
            entries: vec![DefinedNameProjection {
                scope: DefinedNameScopeProjection::Workbook,
                name: "Rate".to_string(),
                target: DefinedNameTargetProjection::Static(GridRectProjection {
                    top_row: 2,
                    left_col: 2,
                    bottom_row: 2,
                    right_col: 2,
                }),
                is_dynamic: false,
            }],
        };
        let result = apply_delta(
            &mut mirror,
            &delta(
                3,
                4,
                vec![WorkspaceDeltaChange::DefinedNamesChanged(catalog.clone())],
            ),
        );
        assert!(result.is_ok());
        assert_eq!(mirror.projection_seq, 4);
        assert_eq!(mirror.defined_names, catalog);
    }

    #[test]
    fn apply_delta_patches_calc_state_changed_in_place() {
        // CalcStateChanged carries the entire new calc-mode/recalc projection
        // (H5): the mirror replaces `WorkspaceState::workbook_calc` in place,
        // without a snapshot, matching DefinedNamesChanged's shape.
        use crate::workspace::{
            CalcModeProjection, SheetCalcSummaryProjection, WorkbookCalcProjection,
        };

        let mut mirror = mirror_at(3);
        assert_eq!(mirror.workbook_calc, None);

        let workbook_calc = WorkbookCalcProjection {
            mode: CalcModeProjection::Manual,
            last_recalc_tick: Some(42),
            sheets: vec![SheetCalcSummaryProjection {
                grid_node_id: NodeId::new("Sheet1"),
                dirty: true,
            }],
        };
        let result = apply_delta(
            &mut mirror,
            &delta(
                3,
                4,
                vec![WorkspaceDeltaChange::CalcStateChanged(
                    workbook_calc.clone(),
                )],
            ),
        );
        assert!(result.is_ok());
        assert_eq!(mirror.projection_seq, 4);
        assert_eq!(mirror.workbook_calc, Some(workbook_calc));
    }

    #[test]
    fn apply_delta_patches_sheets_changed_in_place() {
        // SheetsChanged carries the entire new sheet list (Phase 1 Part A): the
        // mirror replaces `WorkspaceState::sheets` in place, without a snapshot,
        // matching DefinedNamesChanged's shape.
        let mut mirror = mirror_at(3);
        assert!(mirror.sheets.is_empty());

        let sheets = vec![
            SheetProjection {
                grid_node_id: NodeId::new("sheet:1"),
                display_name: "Sheet1".to_string(),
                position: 0,
            },
            SheetProjection {
                grid_node_id: NodeId::new("sheet:2"),
                display_name: "Sheet2".to_string(),
                position: 1,
            },
        ];
        let result = apply_delta(
            &mut mirror,
            &delta(
                3,
                4,
                vec![WorkspaceDeltaChange::SheetsChanged(sheets.clone())],
            ),
        );
        assert!(result.is_ok());
        assert_eq!(mirror.projection_seq, 4);
        assert_eq!(mirror.sheets, sheets);
    }

    #[test]
    fn apply_delta_resyncs_on_gap_and_leaves_the_mirror_untouched() {
        let mut mirror = mirror_at(4);
        let err = apply_delta(
            &mut mirror,
            &delta(6, 7, vec![WorkspaceDeltaChange::CalcRun(sample_calc_run())]),
        )
        .unwrap_err();
        assert_eq!(
            err,
            ResyncReason::SequenceGap {
                expected: 4,
                got: 6
            }
        );
        assert_eq!(mirror.projection_seq, 4, "mirror must not advance on a gap");
        assert_eq!(mirror.last_run, None, "no partial patch on a gap");
    }

    #[test]
    fn apply_delta_resyncs_on_an_unrepresentable_change_without_partial_patch() {
        let mut mirror = mirror_at(4);
        // A mixed delta: a patchable CalcRun *and* an unrepresentable value
        // change. The whole delta must be rejected — no partial patch.
        let err = apply_delta(
            &mut mirror,
            &delta(
                4,
                5,
                vec![
                    WorkspaceDeltaChange::CalcRun(sample_calc_run()),
                    WorkspaceDeltaChange::ValuesChanged(Vec::new()),
                ],
            ),
        )
        .unwrap_err();
        assert_eq!(
            err,
            ResyncReason::UnrepresentableChange {
                kind: "values_changed"
            }
        );
        assert_eq!(mirror.projection_seq, 4);
        assert_eq!(mirror.last_run, None);
    }

    #[test]
    fn for_receipt_ships_a_snapshot_only_when_the_delta_needs_one() {
        let state = mirror_at(2);
        let applicable = IntentReceipt::accepted().with_delta(delta(
            1,
            2,
            vec![WorkspaceDeltaChange::ClipboardChanged(None)],
        ));
        assert!(
            SessionResponse::for_receipt(1, applicable, &state, None)
                .snapshot
                .is_none()
        );

        let needs_snapshot = IntentReceipt::accepted().with_delta(delta(
            1,
            2,
            vec![WorkspaceDeltaChange::ValuesChanged(Vec::new())],
        ));
        assert!(
            SessionResponse::for_receipt(2, needs_snapshot, &state, None)
                .snapshot
                .is_some()
        );
    }

    #[test]
    fn queue_coalesces_deferred_edits_per_node_and_keeps_other_order() {
        let mut queue = PendingIntentQueue::new();
        let edit = |node: &str, content: &str| WorkspaceIntent::EditContentDeferred {
            node: NodeId::new(node),
            content: content.to_string(),
        };

        assert_eq!(queue.enqueue(1, edit("A", "1")), None);
        assert_eq!(queue.enqueue(2, edit("B", "x")), None);
        // A newer edit to A supersedes seq 1.
        assert_eq!(queue.enqueue(3, edit("A", "12")), Some(1));
        // A non-deferred intent keeps its place and never coalesces.
        assert_eq!(queue.enqueue(4, WorkspaceIntent::Recalculate), None);

        assert_eq!(queue.coalesced_count(), 1);
        let drained = queue.drain();
        // A (latest), B, Recalculate — A holds its original slot with seq 3.
        assert_eq!(drained.len(), 3);
        assert_eq!(drained[0].0, 3);
        assert!(
            matches!(&drained[0].1, WorkspaceIntent::EditContentDeferred { content, .. } if content == "12")
        );
        assert!(matches!(drained[2].1, WorkspaceIntent::Recalculate));
        assert!(queue.is_empty());
    }
}

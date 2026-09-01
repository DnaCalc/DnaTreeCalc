//! The Calc degrade-path host adapter — the seam the bridge does NOT cross.
//!
//! `dnacalc-bridge`'s `FormulaBridgeDegrade` emits semantic [`BridgeEvent`]s
//! only; this module maps a commit into `WorkspaceIntent::EnterGridCell` and
//! reads the host's three-way outcome back off the [`IntentReceipt`]
//! (Literal / Formula / Cleared, or a typed rejection). All engine truth comes
//! from `dnacalc-host-core` through the dispatcher — the skin never classifies
//! `=`-vs-literal itself (SHELL_SPEC §6 layering law).
//!
//! The same seam carries the document commands (W011, dtc-j7n8.8): a shell
//! that holds `.xlsx` bytes builds [`open_xlsx_command`] / [`save_xlsx_command`]
//! and reads the typed [`OpenOutcome`] / [`SaveOutcome`] back off the
//! dispatcher's result through [`interpret_open_outcome`] /
//! [`interpret_save_outcome`] — mirroring [`interpret_receipt`]. Every name is
//! reached through `dnatreecalc_host`'s re-exports: the app never takes
//! host-core as a normal dependency, and no skin ever calls OxDoc, OxCalc or a
//! file API (commands enter through the host dispatcher only).

use dnacalc_skin_ir::IntentReceipt;
use dnacalc_skin_ir::identity::NodeId;
use dnacalc_skin_ir::intent::{IntentError, WorkspaceDeltaChange, WorkspaceIntent};
use dnacalc_skin_ir::workspace::{
    GridEntryDiagnosticProjection, GridEntryOutcomeProjection, NodeValueProjection,
};
use dnatreecalc_host::app::WorkbookHostCommandError;
use dnatreecalc_host::{
    HostCommand, HostCommandError, HostCommandOutcome, LoadRecalcPath, WorkbookSessionError,
};

/// The demo cell the degrade editor authors. `A6` (row 6, col 1) is empty in
/// the two-sheet demo workbook (`A1..A5`/`B1..B5` are seeded), so authoring it
/// exercises the entry verb without clobbering the demo's live formulas.
pub const TARGET_ROW: u32 = 6;
pub const TARGET_COL: u32 = 1;

/// Build the single authored-entry intent for the target cell (the degrade
/// path's one entry verb, SHELL_SPEC §6).
#[must_use]
pub fn enter_grid_cell_intent(grid: NodeId, text: String) -> WorkspaceIntent {
    WorkspaceIntent::EnterGridCell {
        grid,
        row: TARGET_ROW,
        col: TARGET_COL,
        text,
    }
}

/// The honestly-rendered result of one commit through the entry verb — the
/// host's three-way success plus the typed-rejection case. Nothing here is
/// inferred by the skin; every arm is read from the host receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellOutcome {
    /// The text classified as a literal; carries the rendered value.
    Literal { value: String },
    /// The text classified as a formula; carries the value plus any
    /// not-yet-resolved defined names (a first-class success field).
    Formula {
        value: String,
        unresolved: Vec<String>,
    },
    /// Empty commit / clear (Excel's empty-commit-clears contract).
    Cleared,
    /// The entry verb rejected the text; carries the typed diagnostics the
    /// degrade editor underlines from.
    Rejected(Vec<GridEntryDiagnosticProjection>),
    /// The receipt carried neither a grid-entry change nor a typed rejection
    /// (never expected on this path — surfaced honestly rather than guessed).
    NoChange,
}

impl CellOutcome {
    /// A short, stable label for the outcome — the `data-outcome` a test reads
    /// and the chip the UI shows.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            CellOutcome::Literal { .. } => "literal",
            CellOutcome::Formula { .. } => "formula",
            CellOutcome::Cleared => "cleared",
            CellOutcome::Rejected(_) => "rejected",
            CellOutcome::NoChange => "no-change",
        }
    }
}

/// Interpret a dispatcher receipt for an `EnterGridCell` commit into the
/// three-way outcome (plus rejection). Reads host truth only.
#[must_use]
pub fn interpret_receipt(receipt: &IntentReceipt) -> CellOutcome {
    if !receipt.accepted {
        return match &receipt.error {
            Some(IntentError::GridEntryRejected { diagnostics }) => {
                CellOutcome::Rejected(diagnostics.clone())
            }
            Some(other) => CellOutcome::Rejected(vec![GridEntryDiagnosticProjection {
                message: format!("{other:?}"),
                span: None,
            }]),
            None => CellOutcome::Rejected(vec![GridEntryDiagnosticProjection {
                message: "rejected without a diagnostic".to_string(),
                span: None,
            }]),
        };
    }
    for change in &receipt.delta.changes {
        if let WorkspaceDeltaChange::GridCellEntered { outcome, .. } = change {
            return match outcome {
                GridEntryOutcomeProjection::Literal { value } => CellOutcome::Literal {
                    value: node_value_display(value),
                },
                GridEntryOutcomeProjection::Formula {
                    unresolved_names,
                    value,
                } => CellOutcome::Formula {
                    value: node_value_display(value),
                    unresolved: unresolved_names.clone(),
                },
                GridEntryOutcomeProjection::Cleared => CellOutcome::Cleared,
            };
        }
    }
    CellOutcome::NoChange
}

/// A compact display string for a projected cell value (the outcome chip's
/// value text).
#[must_use]
pub fn node_value_display(value: &NodeValueProjection) -> String {
    match value {
        NodeValueProjection::Unevaluated => "(unevaluated)".to_string(),
        NodeValueProjection::Pending => "(pending)".to_string(),
        NodeValueProjection::Scalar(text) => text.clone(),
        NodeValueProjection::Number { display, .. } => display.clone(),
        NodeValueProjection::Text(text) => text.clone(),
        NodeValueProjection::Logical { display, .. } => display.clone(),
        NodeValueProjection::Empty => "(empty)".to_string(),
        NodeValueProjection::Missing => "(missing)".to_string(),
        NodeValueProjection::Reference { target } => target.clone(),
        other => format!("{other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Document commands (W011, dtc-j7n8.8): open `.xlsx` bytes / save to bytes.
// ---------------------------------------------------------------------------

/// The open command for a byte buffer a shell already holds (a file dialog,
/// drag-drop, a fetch). `name` is the user-facing document name — the file
/// name, typically — or `None` when the bytes arrived anonymously.
#[must_use]
pub fn open_xlsx_command(bytes: Vec<u8>, name: Option<String>) -> HostCommand {
    HostCommand::OpenXlsxBytes { bytes, name }
}

/// The save command for the active workbook: the outcome carries the complete
/// `.xlsx` package as bytes the shell persists (the session keeps the package
/// it was opened from).
#[must_use]
pub fn save_xlsx_command() -> HostCommand {
    HostCommand::SaveActiveXlsx
}

/// Why a document command was refused — read from the typed error chain, one
/// arm per refusing layer, so the UI can show the refusal structurally. The
/// command-level `UnsupportedByModel { model, command }` is mapped explicitly
/// (it differs in shape from the intent-level
/// `IntentError::UnsupportedByModel { intent, model }`); OxDoc's and the
/// engine's typed errors are carried by their `Display` text (the app has no
/// OxDoc/OxCalc dependency to name their types).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandRejection {
    /// The active document's model family has no such command (e.g. save on
    /// a `RichTree` session).
    UnsupportedByModel { model: String, command: String },
    /// The workbook was not opened from `.xlsx` bytes (the in-memory demo),
    /// so there is no package to round-trip a save against.
    NoBackingSource,
    /// OxDoc rejected the package: corrupt bytes / a missing part on open, or
    /// an edit outside its round-trip policy (a cell add, a formula-text
    /// change) on save.
    Xlsx { message: String },
    /// The engine rejected the operation (a stream/sink mismatch on ingest,
    /// a projection failure on save).
    Engine { message: String },
    /// The dispatcher's session was not reachable on the calling thread.
    SessionUnavailable,
    /// A workbook-session error this adapter has no dedicated arm for
    /// (`WorkbookSessionError` is not `#[non_exhaustive]`; this is the
    /// wildcard the orchestrator asked for so a new upstream arm is shown,
    /// not dropped).
    Other { message: String },
}

impl CommandRejection {
    /// A short, stable label for the refusal (the chip the UI shows).
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            CommandRejection::UnsupportedByModel { .. } => "unsupported-by-model",
            CommandRejection::NoBackingSource => "no-backing-source",
            CommandRejection::Xlsx { .. } => "xlsx",
            CommandRejection::Engine { .. } => "engine",
            CommandRejection::SessionUnavailable => "session-unavailable",
            CommandRejection::Other { .. } => "other",
        }
    }
}

/// Map the dispatcher's typed command error into a [`CommandRejection`].
#[must_use]
pub fn command_rejection(error: &WorkbookHostCommandError) -> CommandRejection {
    match error {
        WorkbookHostCommandError::Command(HostCommandError::UnsupportedByModel {
            model,
            command,
        }) => CommandRejection::UnsupportedByModel {
            model: (*model).to_string(),
            command: (*command).to_string(),
        },
        WorkbookHostCommandError::Command(HostCommandError::Workbook(workbook)) => match workbook {
            WorkbookSessionError::NoBackingSource => CommandRejection::NoBackingSource,
            WorkbookSessionError::Xlsx(xlsx) => CommandRejection::Xlsx {
                message: xlsx.to_string(),
            },
            WorkbookSessionError::OxCalc(engine) => CommandRejection::Engine {
                message: engine.to_string(),
            },
            other => CommandRejection::Other {
                message: other.to_string(),
            },
        },
        WorkbookHostCommandError::SessionUnavailable { .. } => CommandRejection::SessionUnavailable,
    }
}

/// The honestly-rendered result of an open command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenOutcome {
    /// The bytes opened through OxDoc and ingested into the engine; the active
    /// document is now this workbook. The counts are the engine's own load
    /// report (`cells` = literal cells, `formulas_bound` = formula cells bound
    /// into the calc graph) and `recalc_path` is the file's own `calcPr`
    /// disposition (`Automatic` open-recalc vs `Manual` render-from-cache).
    Opened {
        name: Option<String>,
        sheet_count: usize,
        cells: u32,
        formulas_bound: u32,
        recalc_path: LoadRecalcPath,
    },
    /// The command was refused; the previous document is still active.
    Rejected(CommandRejection),
    /// The command executed but answered with an outcome that is not
    /// `Opened` (never expected on this path — surfaced, not guessed).
    Unexpected { outcome: String },
}

impl OpenOutcome {
    /// A short, stable label for the outcome (the chip the UI shows).
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            OpenOutcome::Opened { .. } => "opened",
            OpenOutcome::Rejected(_) => "rejected",
            OpenOutcome::Unexpected { .. } => "unexpected",
        }
    }
}

/// The honestly-rendered result of a save command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    /// OxDoc wrote the complete `.xlsx` package: `bytes` is the shell's to
    /// persist; `ledger_subjects` names every part OxDoc's save ledger
    /// accounted for (the ledger's dispositions are OxDoc data the app cannot
    /// name — see `SEAM-OXDOC-LEDGER` below).
    Saved {
        bytes: Vec<u8>,
        ledger_subjects: Vec<String>,
    },
    /// The command was refused; the live model is untouched.
    Rejected(CommandRejection),
    /// The command executed but answered with an outcome that is not `Saved`
    /// (never expected on this path — surfaced, not guessed).
    Unexpected { outcome: String },
}

impl SaveOutcome {
    /// A short, stable label for the outcome (the chip the UI shows).
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            SaveOutcome::Saved { .. } => "saved",
            SaveOutcome::Rejected(_) => "rejected",
            SaveOutcome::Unexpected { .. } => "unexpected",
        }
    }
}

/// Interpret the dispatcher's result for an [`open_xlsx_command`].
#[must_use]
pub fn interpret_open_outcome(
    result: Result<HostCommandOutcome, WorkbookHostCommandError>,
) -> OpenOutcome {
    match result {
        // Two-dot rest: `Opened` is `#[non_exhaustive]` and also carries
        // OxDoc's load ledger, which the app has no type for.
        Ok(HostCommandOutcome::Opened {
            name,
            sheet_count,
            cells,
            formulas_bound,
            recalc_path,
            ..
        }) => OpenOutcome::Opened {
            name,
            sheet_count,
            cells,
            formulas_bound,
            recalc_path,
        },
        Ok(other) => OpenOutcome::Unexpected {
            outcome: format!("{other:?}"),
        },
        Err(error) => OpenOutcome::Rejected(command_rejection(&error)),
    }
}

/// Interpret the dispatcher's result for a [`save_xlsx_command`]. Takes the
/// result by value so the package bytes move to the caller without a copy.
#[must_use]
pub fn interpret_save_outcome(
    result: Result<HostCommandOutcome, WorkbookHostCommandError>,
) -> SaveOutcome {
    match result {
        Ok(HostCommandOutcome::Saved { bytes, save_ledger }) => SaveOutcome::Saved {
            bytes,
            // SEAM-OXDOC-LEDGER: only the ledger's `subject` names are carried.
            // Its `disposition` (`Dropped` is the visible-loss signal) is an
            // `oxdoc_model` enum host-core does not re-export, and this app
            // must not take OxDoc directly — typing it here is the successor
            // bead filed with dtc-j7n8.8.
            ledger_subjects: save_ledger
                .entries
                .iter()
                .map(|entry| entry.subject.clone())
                .collect(),
        },
        Ok(other) => SaveOutcome::Unexpected {
            outcome: format!("{other:?}"),
        },
        Err(error) => SaveOutcome::Rejected(command_rejection(&error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dnacalc_host_core::{DocumentSession, build_demo_workbook, sheet_grid_node_id};

    /// Drive the REAL host-core demo workbook through the exact intent the
    /// degrade adapter builds, and prove all three success outcomes plus the
    /// typed rejection flow through `interpret_receipt`.
    #[test]
    fn three_way_outcome_flows_from_the_real_engine() {
        let session = build_demo_workbook().expect("demo workbook");
        let sheet1 = session.sheets().unwrap()[0].node_id;
        let grid = sheet_grid_node_id(sheet1);
        let mut document = DocumentSession::Workbook(session);

        // Literal.
        let receipt = document.dispatch(enter_grid_cell_intent(grid.clone(), "42".to_string()));
        assert_eq!(
            interpret_receipt(&receipt),
            CellOutcome::Literal {
                value: "42".to_string()
            }
        );

        // Formula (references seeded cells A1 + A5 = 1 + 5 = 6).
        let receipt = document.dispatch(enter_grid_cell_intent(grid.clone(), "=A1+A5".to_string()));
        match interpret_receipt(&receipt) {
            CellOutcome::Formula { value, unresolved } => {
                assert_eq!(value, "6");
                assert!(unresolved.is_empty());
            }
            other => panic!("expected Formula outcome, got {other:?}"),
        }

        // Cleared (empty commit).
        let receipt = document.dispatch(enter_grid_cell_intent(grid.clone(), String::new()));
        assert_eq!(interpret_receipt(&receipt), CellOutcome::Cleared);

        // Rejected (an unparseable formula) — typed diagnostics, no mutation.
        let receipt = document.dispatch(enter_grid_cell_intent(grid, "=1+".to_string()));
        match interpret_receipt(&receipt) {
            CellOutcome::Rejected(diagnostics) => assert!(!diagnostics.is_empty()),
            other => panic!("expected Rejected outcome, got {other:?}"),
        }
    }

    /// The W011 app-level command tests below drive the same
    /// `WorkbookHostDispatcher` the app mounts, over Leptos signals, against
    /// the COMMITTED fixture binary read from disk (the app click-through's
    /// byte source; host-core's own tests zip the readable parts instead).
    /// Native only: they read the file system.
    #[cfg(not(target_arch = "wasm32"))]
    mod commands {
        use super::*;
        use dnacalc_skin_ir::intent::{Dispatcher, WorkspaceDelta};
        use dnacalc_skin_ir::selection::SelectionState;
        use dnacalc_skin_ir::workspace::{
            GridAuthoredKindProjection, GridCellProjection, WorkspaceState,
        };
        use dnatreecalc_host::app::WorkbookHostDispatcher;
        use leptos::prelude::*;
        use std::path::{Path, PathBuf};

        /// Repo-relative location of the committed fixture binary, walked from
        /// this crate's manifest dir (`src/dnacalc-app`). Relative on purpose.
        const FIXTURE_XLSX_REL: &str = "../../fixtures/w011/a1_times_three.xlsx";

        fn fixture_path() -> PathBuf {
            Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_XLSX_REL)
        }

        fn fixture_bytes() -> Vec<u8> {
            let path = fixture_path();
            let bytes = std::fs::read(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            println!(
                "W011 app [fixture] {} bytes read from {}",
                bytes.len(),
                path.display()
            );
            bytes
        }

        /// The three signals the app mounts the dispatcher over.
        struct Signals {
            workspace: RwSignal<WorkspaceState>,
            latest_delta: RwSignal<WorkspaceDelta>,
            selection: RwSignal<SelectionState>,
        }

        fn signals() -> Signals {
            Signals {
                workspace: RwSignal::new(WorkspaceState::default()),
                latest_delta: RwSignal::new(WorkspaceDelta::unchanged(0)),
                selection: RwSignal::new(SelectionState::default()),
            }
        }

        /// Sheet1's grid id plus its projected `A1` and `B1`, read from the
        /// PUBLISHED workspace signal (what a lens sees), never from the
        /// session directly.
        fn sheet1_a1_b1(
            state: &WorkspaceState,
        ) -> (NodeId, GridCellProjection, GridCellProjection) {
            assert_eq!(
                state.sheets.len(),
                1,
                "the loaded fixture has exactly one sheet: {:?}",
                state.sheets
            );
            assert_eq!(state.sheets[0].display_name, "Sheet1");
            let grid_id = state.sheets[0].grid_node_id.clone();
            let grid = state.grids.get(&grid_id).unwrap_or_else(|| {
                panic!(
                    "Sheet1's grid {grid_id:?} is published; grids = {:?}",
                    state.grids.keys().collect::<Vec<_>>()
                )
            });
            let cell = |row: u32, col: u32| {
                grid.cells
                    .iter()
                    .find(|cell| cell.row == row && cell.col == col)
                    .cloned()
                    .unwrap_or_else(|| {
                        panic!("no published cell at ({row}, {col}): {:#?}", grid.cells)
                    })
            };
            (grid_id, cell(1, 1), cell(1, 2))
        }

        fn log_cell(stage: &str, label: &str, cell: &GridCellProjection) {
            println!(
                "W011 app [{stage}] {label} kind={:?} literal_text={:?} source_text={:?} \
                 value={} provenance={:?}",
                cell.authored.as_ref().map(|authored| authored.kind),
                cell.authored
                    .as_ref()
                    .and_then(|authored| authored.literal_text.as_deref()),
                cell.authored
                    .as_ref()
                    .and_then(|authored| authored.source_text.as_deref()),
                node_value_display(&cell.value),
                cell.provenance,
            );
        }

        /// Assert the published `A1`/`B1` of the loaded fixture: `A1` a
        /// literal with `a1_text`, `B1` the formula `=A1*3`, both with the
        /// given display values (authored kinds first, then values — the
        /// token-mismatch blank order).
        fn assert_a1_b1(
            stage: &str,
            state: &WorkspaceState,
            a1_text: &str,
            b1_value: &str,
        ) -> NodeId {
            let (grid_id, a1, b1) = sheet1_a1_b1(state);
            log_cell(stage, "A1", &a1);
            log_cell(stage, "B1", &b1);
            let a1_authored = a1
                .authored
                .as_ref()
                .expect("A1 carries authored metadata (None = token-mismatch blank)");
            let b1_authored = b1
                .authored
                .as_ref()
                .expect("B1 carries authored metadata (None = token-mismatch blank)");
            assert_eq!(
                a1_authored.kind,
                GridAuthoredKindProjection::Literal,
                "[{stage}] A1 kind"
            );
            assert_eq!(
                a1_authored.literal_text.as_deref(),
                Some(a1_text),
                "[{stage}] A1 text"
            );
            assert_eq!(
                b1_authored.kind,
                GridAuthoredKindProjection::Formula,
                "[{stage}] B1 kind"
            );
            assert_eq!(
                b1_authored.source_text.as_deref(),
                Some("=A1*3"),
                "[{stage}] B1 keeps its formula text"
            );
            assert_eq!(node_value_display(&a1.value), a1_text, "[{stage}] A1 value");
            assert_eq!(
                node_value_display(&b1.value),
                b1_value,
                "[{stage}] B1 = A1*3"
            );
            grid_id
        }

        /// dtc-j7n8.8 acceptance (1) — the end-to-end app-level proof over the
        /// committed fixture binary, through the commands and intents the app
        /// itself issues: mount the demo (as `CalcApp` does) -> `OpenXlsxBytes`
        /// replaces it with the fixture (published `A1` = 7, `B1` = 21, caret
        /// on Sheet1) -> `EnterGridCell A1 "10"` (published `B1` = 30) ->
        /// `SaveActiveXlsx` returns bytes and leaves the live model alone ->
        /// a fresh open command on THOSE bytes publishes `A1` authored 10 and
        /// `B1` authored `=A1*3` = 30.
        ///
        /// The reopen here goes through the engine (the fixture is
        /// `Automatic`, so open-recalc runs); the FILE-level proof that the
        /// saved `B1` carries cached `<v>` 30 — not the stale 21 — is
        /// host-core's `execute_save_active_xlsx_after_edit_returns_bytes_with_cached_30`
        /// (dtc-j7n8.7), which walks the saved bytes as raw OxDoc events with
        /// no engine. This test proves the same bytes, produced through the
        /// app's dispatcher, close the loop the user sees.
        #[test]
        fn app_opens_fixture_edits_and_saves_through_commands() {
            let _owner = Owner::new();
            let signals = signals();
            let dispatcher = WorkbookHostDispatcher::new_demo(
                signals.workspace,
                signals.latest_delta,
                signals.selection,
                None,
            )
            .expect("the demo mounts as CalcApp does");
            let demo = signals.workspace.get_untracked();
            assert_eq!(demo.sheets.len(), 2, "the demo publishes its two sheets");
            let seq_before_open = demo.projection_seq;

            // Open: the fixture replaces the demo as the active document.
            let outcome = interpret_open_outcome(dispatcher.execute_host_command(
                open_xlsx_command(fixture_bytes(), Some("a1_times_three.xlsx".to_string())),
            ));
            println!("W011 app [open] outcome = {outcome:?}");
            assert_eq!(
                outcome,
                OpenOutcome::Opened {
                    name: Some("a1_times_three.xlsx".to_string()),
                    sheet_count: 1,
                    cells: 1,
                    formulas_bound: 1,
                    recalc_path: LoadRecalcPath::Automatic,
                }
            );
            let opened = signals.workspace.get_untracked();
            assert!(
                opened.projection_seq > seq_before_open,
                "the open republished the snapshot ({} > {seq_before_open})",
                opened.projection_seq
            );
            let grid_id = assert_a1_b1("opened", &opened, "7", "21");
            assert_eq!(
                signals.selection.get_untracked().primary.as_ref(),
                Some(&grid_id),
                "the caret moved to the loaded workbook's Sheet1 grid"
            );
            assert_eq!(
                signals.latest_delta.get_untracked().to_seq,
                opened.projection_seq,
                "the delta signal carries the swapped document's projection seq"
            );

            // Edit: A1 7 -> 10 through the intent path; B1 recalcs live.
            let receipt = dispatcher.dispatch(WorkspaceIntent::EnterGridCell {
                grid: grid_id.clone(),
                row: 1,
                col: 1,
                text: "10".to_string(),
            });
            assert_eq!(
                interpret_receipt(&receipt),
                CellOutcome::Literal {
                    value: "10".to_string()
                },
                "A1 -> 10 is accepted as a literal: {:?}",
                receipt.error
            );
            let edited = signals.workspace.get_untracked();
            assert_a1_b1("edited", &edited, "10", "30");

            // Save: bytes come back; the live model is untouched.
            let saved =
                interpret_save_outcome(dispatcher.execute_host_command(save_xlsx_command()));
            let SaveOutcome::Saved {
                bytes: saved_bytes,
                ledger_subjects,
            } = saved
            else {
                panic!("expected Saved, got {saved:?}");
            };
            println!(
                "W011 app [save] {} bytes; ledger subjects = {ledger_subjects:?}",
                saved_bytes.len()
            );
            assert!(!saved_bytes.is_empty(), "the save produced package bytes");
            assert!(
                !ledger_subjects.is_empty(),
                "OxDoc's save ledger accounted for the package parts"
            );
            let after_save = signals.workspace.get_untracked();
            assert_eq!(
                after_save.projection_seq, edited.projection_seq,
                "a save publishes nothing (the session is neither replaced nor mutated)"
            );
            assert_a1_b1("after-save", &after_save, "10", "30");

            // Reopen the SAVED bytes through a fresh open command.
            let reopened = interpret_open_outcome(dispatcher.execute_host_command(
                open_xlsx_command(saved_bytes, Some("a1_times_three_saved.xlsx".to_string())),
            ));
            println!("W011 app [reopen] outcome = {reopened:?}");
            assert_eq!(
                reopened,
                OpenOutcome::Opened {
                    name: Some("a1_times_three_saved.xlsx".to_string()),
                    sheet_count: 1,
                    cells: 1,
                    formulas_bound: 1,
                    recalc_path: LoadRecalcPath::Automatic,
                },
                "the saved bytes open through OxDoc and ingest into the engine"
            );
            let reloaded = signals.workspace.get_untracked();
            assert!(reloaded.projection_seq > after_save.projection_seq);
            assert_a1_b1("reopened", &reloaded, "10", "30");
        }

        /// `new_from_xlsx_bytes` is the mount entry point for a shell that
        /// already holds the bytes: the dispatcher comes up on the fixture
        /// (published `A1` = 7, `B1` = 21) with the caret on Sheet1.
        #[test]
        fn new_from_xlsx_bytes_mounts_the_fixture_and_selects_sheet1() {
            let _owner = Owner::new();
            let signals = signals();
            let _dispatcher = WorkbookHostDispatcher::new_from_xlsx_bytes(
                &fixture_bytes(),
                Some("a1_times_three.xlsx".to_string()),
                signals.workspace,
                signals.latest_delta,
                signals.selection,
                None,
            )
            .expect("the fixture bytes mount");
            let state = signals.workspace.get_untracked();
            let grid_id = assert_a1_b1("mounted", &state, "7", "21");
            assert_eq!(
                signals.selection.get_untracked().primary.as_ref(),
                Some(&grid_id)
            );
        }

        /// Typed refusals stay typed through the adapter: a save on the
        /// in-memory demo (no backing package) is `NoBackingSource`, the demo
        /// is left exactly as it was, and a corrupt buffer is refused by OxDoc
        /// with the previous document still active.
        #[test]
        fn refusals_are_typed_and_leave_the_document_alone() {
            let _owner = Owner::new();
            let signals = signals();
            let dispatcher = WorkbookHostDispatcher::new_demo(
                signals.workspace,
                signals.latest_delta,
                signals.selection,
                None,
            )
            .expect("demo");
            let before = signals.workspace.get_untracked();

            let saved =
                interpret_save_outcome(dispatcher.execute_host_command(save_xlsx_command()));
            println!("W011 app [save on demo] outcome = {saved:?}");
            assert_eq!(
                saved,
                SaveOutcome::Rejected(CommandRejection::NoBackingSource)
            );
            assert_eq!(saved.label(), "rejected");

            let opened = interpret_open_outcome(dispatcher.execute_host_command(
                open_xlsx_command(b"not a zip".to_vec(), Some("junk.xlsx".to_string())),
            ));
            println!("W011 app [open junk] outcome = {opened:?}");
            match &opened {
                OpenOutcome::Rejected(CommandRejection::Xlsx { message }) => {
                    assert!(!message.is_empty(), "OxDoc's refusal is carried")
                }
                other => panic!("expected an OxDoc (Xlsx) rejection, got {other:?}"),
            }

            let after = signals.workspace.get_untracked();
            assert_eq!(
                after.projection_seq, before.projection_seq,
                "nothing was republished"
            );
            assert_eq!(
                after.sheets, before.sheets,
                "the demo is still the active document"
            );
        }

        /// The command-level `UnsupportedByModel { model, command }` maps
        /// explicitly (it is not the intent-level shape).
        #[test]
        fn unsupported_by_model_maps_explicitly() {
            let error = WorkbookHostCommandError::Command(HostCommandError::UnsupportedByModel {
                model: "RichTree",
                command: "SaveActiveXlsx",
            });
            assert_eq!(
                command_rejection(&error),
                CommandRejection::UnsupportedByModel {
                    model: "RichTree".to_string(),
                    command: "SaveActiveXlsx".to_string(),
                }
            );
            assert_eq!(
                command_rejection(&WorkbookHostCommandError::SessionUnavailable { session_id: 7 }),
                CommandRejection::SessionUnavailable
            );
        }
    }
}

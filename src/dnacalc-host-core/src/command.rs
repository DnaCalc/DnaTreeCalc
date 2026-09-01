//! `HostCommand` — the typed host command surface, and the projection
//! publication seam.
//!
//! Boundary rule (proof doc §`dnacalc-host-core`): `WorkspaceIntent` mutates
//! the *open model*; `HostCommand` manages *documents, files, and layout*. H2
//! stood up the skeleton with the one arm it could execute end to end —
//! [`HostCommand::DispatchWorkspaceIntent`] — and the publication seam. The
//! file/xlsx arms land with the W011 successor slice (epic `dtc-j7n8`):
//! OxCalc's R6 `oxdoc-model` ingest has landed upstream and host-core takes
//! `oxdoc_model`/`oxdoc_xlsx` directly (dtc-j7n8.1), so what remains is host
//! wiring of real `.xlsx` bytes through OxDoc — not a new engine, and never a
//! raw ZIP/XML bypass. [`HostCommand::OpenXlsxBytes`] is the open arm
//! (dtc-j7n8.3); [`HostCommand::SaveActiveXlsx`] is the save arm (dtc-j7n8.7:
//! the engine hands OxDoc its whole-model projection with fresh formula
//! caches to round-trip). The boundary rule above stands for both; the enum
//! is `#[non_exhaustive]` so the remaining file/layout arms are not a
//! breaking change.

use dnacalc_skin_ir::{IntentReceipt, WorkspaceIntent};
use oxcalc_core::oxdoc_ingest::LoadRecalcPath;
use oxdoc_xlsx::model::DocumentFidelityLedger;

use crate::workbook::WorkbookSessionError;

/// The typed host command surface: the model-dispatch arm (H2) and the
/// document-open and document-save arms (W011); the remaining file/layout
/// arms land in later beads (marked in the proof doc).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HostCommand {
    /// Route a `WorkspaceIntent` to the active document session.
    DispatchWorkspaceIntent(WorkspaceIntent),
    /// Open a workbook from `.xlsx` bytes through OxDoc (W011, dtc-j7n8.3).
    /// On success the opened workbook **replaces** the active document
    /// session — the previous session drops — and the host owns the OxDoc
    /// source (`WorkbookSession::xlsx_source`) for the later round-trip save.
    /// On failure the active session is left untouched and OxDoc's typed
    /// rejection is returned as [`HostCommandError`].
    ///
    /// `bytes` is a plain owned buffer: the package bytes exactly as the
    /// shell received them (file dialog, drag-drop, fetch). `name` is the
    /// user-facing document name (typically the file name), or `None` when
    /// the bytes arrived anonymously.
    OpenXlsxBytes {
        bytes: Vec<u8>,
        name: Option<String>,
    },
    /// Save the active workbook back to `.xlsx` bytes through OxDoc (W011,
    /// dtc-j7n8.7): the engine projects the whole model with FRESH formula
    /// caches (`project_workbook_model_output`, OxCalc C12) and OxDoc
    /// round-trips it against the package the workbook was opened from
    /// (`write_save_request`) — see [`crate::WorkbookSession::save_xlsx_bytes`]
    /// for the stale-cache trap this closes. On success the outcome is
    /// [`HostCommandOutcome::Saved`] carrying the bytes and OxDoc's save
    /// ledger; the active session is neither replaced nor mutated (the shell
    /// owns file I/O and takes the bytes wherever they go). Typed refusals,
    /// never panics: a `RichTree` session
    /// ([`HostCommandError::UnsupportedByModel`]), a workbook not opened from
    /// bytes ([`WorkbookSessionError::NoBackingSource`]), an edit outside
    /// OxDoc's round-trip policy — a cell add, a formula-text change —
    /// (OxDoc's `XlsxError::UnsupportedRoundTripFeature` inside
    /// [`WorkbookSessionError::Xlsx`], the live model left intact).
    SaveActiveXlsx,
}

/// The outcome of a successfully executed [`HostCommand`]. `#[non_exhaustive]`
/// so the file/layout outcomes can be added without a breaking change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HostCommandOutcome {
    /// A `WorkspaceIntent` was routed to the active session; the session's
    /// typed receipt is carried through verbatim (including the model-family
    /// rejection receipt when a family does not support the intent).
    Dispatched(IntentReceipt),
    /// `OpenXlsxBytes` succeeded: the active session is now the opened
    /// workbook, ingested into the engine (dtc-j7n8.4). `name` echoes the
    /// command's document name; `sheet_count` is the engine's own sheet
    /// enumeration after ingest; `cells` (literal cells folded into authored
    /// truth), `formulas_bound` (formula cells bound into the calc graph), and
    /// `recalc_path` (`Automatic` open-recalc vs `Manual` render-from-cache,
    /// from the file's own `calcPr`) are the salient fields of the engine's
    /// `WorkbookLoadReport` — engine truth, surfaced as-is (the full report is
    /// [`crate::WorkbookSession::load_report`]); and `load_ledger` is OxDoc's
    /// fidelity ledger for the load — what was preserved, projected, or
    /// dropped — carried as typed data so a shell can show it.
    Opened {
        name: Option<String>,
        sheet_count: usize,
        cells: u32,
        formulas_bound: u32,
        recalc_path: LoadRecalcPath,
        load_ledger: DocumentFidelityLedger,
    },
    /// `SaveActiveXlsx` succeeded (dtc-j7n8.7): `bytes` is the complete
    /// `.xlsx` package OxDoc wrote (the caller persists it — the session
    /// keeps the package it was opened from), and `save_ledger` is OxDoc's
    /// fidelity ledger for the save — what was preserved, projected, or
    /// dropped — as typed data so a shell can show it. A `Dropped` entry is
    /// the visible-loss signal (e.g. a stale `xl/calcChain.xml` removed
    /// after a formula-cache refresh; the W011 fixture carries none).
    Saved {
        bytes: Vec<u8>,
        save_ledger: DocumentFidelityLedger,
    },
}

/// A [`HostCommand`] that could not be executed. Every arm carries the typed
/// error of the layer that rejected the command as data — never a formatted
/// string — so a shell can present it structurally (the W011 "engine errors
/// are data" rule, extended to document errors: OxDoc's `XlsxError` travels
/// inside [`WorkbookSessionError::Xlsx`]).
///
/// `DispatchWorkspaceIntent` never produces this error: intent rejections are
/// part of the [`IntentReceipt`] contract and travel inside
/// [`HostCommandOutcome::Dispatched`].
// `large_enum_variant`: the `Workbook` arm wraps `WorkbookSessionError` by
// value (which wraps the engine's `OxCalcDocumentError` by value — the
// convention `workbook.rs` documents), and `UnsupportedByModel` is two
// `&'static str`s. Boxing the workbook arm would break the `#[from]`/`?`
// conversion every `execute` arm relies on and diverge from the sibling error
// shapes for no caller benefit (a command execution is a single host call, not
// a hot inner loop). Kept by-value for cross-session consistency.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, thiserror::Error)]
pub enum HostCommandError {
    /// The workbook session rejected the command — on `OpenXlsxBytes`, OxDoc
    /// rejected the bytes ([`WorkbookSessionError::Xlsx`]) or the engine
    /// rejected loading the document stream into a workbook workspace
    /// (`load_workbook_model`, [`WorkbookSessionError::OxCalc`] — e.g. a
    /// `WorkbookIngestRejected` stream/sink mismatch); on `SaveActiveXlsx`,
    /// the workbook has no backing source
    /// ([`WorkbookSessionError::NoBackingSource`]), the engine could not
    /// project it, or OxDoc's round-trip policy refused the projected edit
    /// ([`WorkbookSessionError::Xlsx`] carrying
    /// `XlsxError::UnsupportedRoundTripFeature`).
    #[error("the workbook session rejected the host command")]
    Workbook(#[from] WorkbookSessionError),
    /// The active session's model family does not support the command
    /// (W011, dtc-j7n8.7): `SaveActiveXlsx` on a `RichTree` session, which
    /// has no workbook and no OxDoc source to round-trip. The command-level
    /// mirror of the `IntentError::UnsupportedByModel` receipt: `model` is
    /// the session's stable model name, `command` the refused arm's name.
    #[error("{command} is not supported by the {model} document model")]
    UnsupportedByModel {
        model: &'static str,
        command: &'static str,
    },
}

/// The publication seam: the host publishes projection deltas/snapshots through
/// this trait, and a transport binds to it. The Leptos adapter binds signals;
/// the worker binds `postMessage`. Host-core stays transport-agnostic and
/// **Leptos-free** by depending only on this trait (proof doc
/// §`dnacalc-host-core`: "Skin IR snapshot and delta publication via a
/// `ProjectionPublisher` seam").
///
/// H2 defines the seam and its `Debug`-friendly recording double; wiring a live
/// publisher into dispatch is a later bead.
pub trait ProjectionPublisher {
    /// Publish an intent receipt (carrying its `WorkspaceDelta`) to the bound
    /// transport.
    fn publish(&self, receipt: &IntentReceipt);
}

/// A recording publication seam for tests: it captures every published receipt
/// in order so a test can assert the publication stream without a live
/// transport. The skin-side double (`RecordingDispatcher`, skin-IR crate) plays
/// the mirror role; this is the host-side publisher double.
#[derive(Debug, Default)]
pub struct RecordingPublisher {
    published: std::sync::Mutex<Vec<IntentReceipt>>,
}

impl RecordingPublisher {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The receipts published so far, in order.
    #[must_use]
    pub fn published(&self) -> Vec<IntentReceipt> {
        self.published.lock().expect("publisher lock").clone()
    }
}

impl ProjectionPublisher for RecordingPublisher {
    fn publish(&self, receipt: &IntentReceipt) {
        self.published
            .lock()
            .expect("publisher lock")
            .push(receipt.clone());
    }
}

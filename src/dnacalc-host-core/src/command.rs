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
//! (dtc-j7n8.3); `SaveActiveXlsx` follows once the engine can hand OxDoc a
//! model output to round-trip (dtc-j7n8.7). The boundary rule above stands
//! for both; the enum is `#[non_exhaustive]` so adding them is not a breaking
//! change.

use dnacalc_skin_ir::{IntentReceipt, WorkspaceIntent};
use oxdoc_xlsx::model::DocumentFidelityLedger;

use crate::workbook::WorkbookSessionError;

/// The typed host command surface: the model-dispatch arm (H2) and the
/// document-open arm (W011); the remaining file/layout arms land in later
/// beads (marked in the proof doc).
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
    /// workbook. `name` echoes the command's document name, `sheet_count` is
    /// what OxDoc's model context enumerated for the package (the engine's own
    /// sheet list follows with ingest, dtc-j7n8.4), and `load_ledger` is
    /// OxDoc's fidelity ledger for the load — what was preserved, projected,
    /// or dropped — carried as typed data so a shell can show it.
    Opened {
        name: Option<String>,
        sheet_count: usize,
        load_ledger: DocumentFidelityLedger,
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
#[derive(Debug, thiserror::Error)]
pub enum HostCommandError {
    /// The workbook session rejected the command — on `OpenXlsxBytes`, OxDoc
    /// rejected the bytes ([`WorkbookSessionError::Xlsx`]) or the engine
    /// rejected standing up the workbook workspace
    /// ([`WorkbookSessionError::OxCalc`]).
    #[error("the workbook session rejected the host command")]
    Workbook(#[from] WorkbookSessionError),
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

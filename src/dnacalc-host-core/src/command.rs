//! `HostCommand` — the typed host command surface, and the projection
//! publication seam.
//!
//! Boundary rule (proof doc §`dnacalc-host-core`): `WorkspaceIntent` mutates
//! the *open model*; `HostCommand` manages *documents, files, and layout*. H2
//! stands up the skeleton with the one arm it can execute end to end —
//! [`HostCommand::DispatchWorkspaceIntent`] — and the publication seam. The
//! file/xlsx arms (`OpenXlsxBytes`, `SaveActiveXlsx`) are a NON-goal of H2 and
//! land with the xlsx lane at R6; the enum is `#[non_exhaustive]` so adding
//! them later is not a breaking change.

use dnacalc_skin_ir::{IntentReceipt, WorkspaceIntent};

/// The typed host command surface. H2 carries only the model-dispatch arm; the
/// document/file/layout arms land in later beads (marked in the proof doc).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HostCommand {
    /// Route a `WorkspaceIntent` to the active document session. This is the
    /// sole executable arm in H2.
    DispatchWorkspaceIntent(WorkspaceIntent),
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

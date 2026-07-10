//! The DNA TreeCalc calculation-worker **message protocol**.
//!
//! The worker runtime itself lives in `dnatreecalc-web`: the browser worker
//! imports the same generated web module as the main app, so the worker entry
//! must be in that crate, behind a context-detecting `start`. This crate is just the
//! serializable message types shared by both sides of the `postMessage`
//! boundary: the main thread sends [`WorkerInbound`], the worker replies
//! [`WorkerOutbound`].

use dnacalc_skin_ir::{SkinIntentEnvelope, SkinIntentReceipt, SkinSnapshot};
use dnatreecalc_host::app::DnaTreeWorkspaceDocument;
use dnatreecalc_skin_framework::{IntentEnvelope, SelectionState, SessionResponse, WorkspaceState};
use serde::{Deserialize, Serialize};

/// Main thread → worker. (Large payloads are boxed to keep the message enum
/// small.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerInbound {
    /// The first message: the initial document, so the worker can build the
    /// session it will own.
    Init {
        document: Box<DnaTreeWorkspaceDocument>,
    },
    /// A sequence-stamped intent to run against the session.
    Intent { envelope: IntentEnvelope },
    /// Shared host-neutral envelope used by in-process, worker, and native transports.
    SharedIntent { envelope: SkinIntentEnvelope },
}

/// Worker → main thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerOutbound {
    /// Sent once after [`WorkerInbound::Init`]: the session's first projection,
    /// for the proxy's initial mirror.
    Ready {
        snapshot: Box<WorkspaceState>,
        selection: Option<SelectionState>,
    },
    /// The result of an [`WorkerInbound::Intent`].
    Response { response: Box<SessionResponse> },
    /// Shared protocol initialization projection.
    SharedReady { snapshot: Box<SkinSnapshot> },
    /// Typed shared protocol receipt.
    SharedResponse {
        receipt: Box<SkinIntentReceipt>,
        snapshot: Option<Box<SkinSnapshot>>,
    },
    /// The worker could not build or run (carries a human-readable reason). The
    /// main thread surfaces this and may fall back to in-process dispatch.
    Failed { message: String },
}

//! The DNA TreeCalc calculation-worker **message protocol**.
//!
//! The worker runtime itself lives in `dnatreecalc-web` (trunk 0.21 builds the
//! web crate in `no-modules` mode for the worker, so the worker entry must be
//! the same crate, behind a context-detecting `start`). This crate is just the
//! serializable message types shared by both sides of the `postMessage`
//! boundary: the main thread sends [`WorkerInbound`], the worker replies
//! [`WorkerOutbound`].

use dnatreecalc_host::app::DnaTreeWorkspaceDocument;
use dnatreecalc_skin_framework::{IntentEnvelope, SelectionState, SessionResponse, WorkspaceState};
use serde::{Deserialize, Serialize};

/// Main thread → worker. (Large payloads are boxed to keep the message enum
/// small.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerInbound {
    /// The first message: the initial document, so the worker can build the
    /// session it will own.
    Init { document: Box<DnaTreeWorkspaceDocument> },
    /// A sequence-stamped intent to run against the session.
    Intent { envelope: IntentEnvelope },
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
    /// The worker could not build or run (carries a human-readable reason). The
    /// main thread surfaces this and may fall back to in-process dispatch.
    Failed { message: String },
}

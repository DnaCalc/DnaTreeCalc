//! The DNA TreeCalc calculation **worker**.
//!
//! This wasm artifact runs the `TreeWorkspaceSession` off the UI thread. The
//! main thread postMessages an [`IntentEnvelope`] (wrapped in [`WorkerInbound`]);
//! the worker runs it through a [`HostSessionExecutor`] — the same executor the
//! in-process path uses — and postMessages back a [`SessionResponse`] (wrapped
//! in [`WorkerOutbound`]). Selection, shared state, undo/redo, persistence, and
//! the persona gate all stay on the main thread; the worker is pure engine.
//!
//! The message protocol types compile on every target so the main thread can
//! share them; the worker event loop is `wasm32`-only.

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

#[cfg(target_arch = "wasm32")]
mod runtime {
    use std::cell::RefCell;
    use std::sync::{Arc, Mutex};

    use dnatreecalc_host::app::{
        DnaTreeWorkspaceDocument, HostSessionExecutor, SessionExecutor, TreeWorkspaceSession,
    };
    use leptos::prelude::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

    use super::{WorkerInbound, WorkerOutbound};

    /// The owned session executor, plus the reactive owner its signals belong
    /// to (kept alive for the worker's lifetime). The worker is single-threaded,
    /// so a thread-local cell is the natural home.
    struct WorkerState {
        _owner: Owner,
        executor: HostSessionExecutor,
    }

    thread_local! {
        static STATE: RefCell<Option<WorkerState>> = const { RefCell::new(None) };
    }

    fn scope() -> DedicatedWorkerGlobalScope {
        js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>()
    }

    fn post(message: &WorkerOutbound) {
        if let Ok(json) = serde_json::to_string(message) {
            let _ = scope().post_message(&JsValue::from_str(&json));
        }
    }

    /// Install the message handler. wasm-bindgen calls this when the worker
    /// module loads.
    #[wasm_bindgen(start)]
    pub fn start() {
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(handle_message);
        scope().set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();
    }

    fn handle_message(event: MessageEvent) {
        let Some(text) = event.data().as_string() else {
            post(&WorkerOutbound::Failed {
                message: "worker message was not a JSON string".to_string(),
            });
            return;
        };
        match serde_json::from_str::<WorkerInbound>(&text) {
            Ok(WorkerInbound::Init { document }) => init_session(*document),
            Ok(WorkerInbound::Intent { envelope }) => STATE.with(|state| {
                match state.borrow().as_ref() {
                    Some(state) => post(&WorkerOutbound::Response {
                        response: Box::new(state.executor.execute(envelope)),
                    }),
                    None => post(&WorkerOutbound::Failed {
                        message: "intent arrived before the worker was initialized".to_string(),
                    }),
                }
            }),
            Err(error) => post(&WorkerOutbound::Failed {
                message: format!("could not decode a worker message: {error}"),
            }),
        }
    }

    fn init_session(document: DnaTreeWorkspaceDocument) {
        // Signals are created under an owner that lives as long as the worker.
        let owner = Owner::new();
        let built = owner.with(|| {
            let (session, _selection) = TreeWorkspaceSession::from_dnatree_document(document)
                .map_err(|error| error.to_string())?;
            Ok::<_, String>(HostSessionExecutor::new(Arc::new(Mutex::new(session))))
        });
        match built {
            Ok(executor) => {
                let snapshot = executor.snapshot();
                STATE.with(|state| {
                    *state.borrow_mut() = Some(WorkerState {
                        _owner: owner,
                        executor,
                    });
                });
                post(&WorkerOutbound::Ready {
                    snapshot: Box::new(snapshot),
                    selection: None,
                });
            }
            Err(message) => post(&WorkerOutbound::Failed { message }),
        }
    }
}

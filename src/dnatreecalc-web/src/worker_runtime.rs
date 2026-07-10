//! The worker side of the calculation boundary.
//!
//! The browser worker imports the same generated module as the main app, giving
//! it a separate wasm instance in a worker global. Both contexts run
//! [`crate::start`], which detects the context — a worker has no `window` — and
//! on the worker side calls
//! [`start_worker`] here. The worker owns the `TreeWorkspaceSession` via a
//! [`HostSessionExecutor`] and answers [`WorkerInbound`] messages with
//! [`WorkerOutbound`] over `postMessage`. No DOM, no Leptos UI — just the
//! reactive `Owner` the executor's signals need.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use dnacalc_skin_ir::{
    ExtensionPlacementProjection, HostCapabilityProjection, HostKindProjection, IntentEnvelope,
    RuntimeProfileProjection, SkinIntent, SkinIntentDiagnostic, SkinIntentReceipt,
    SkinShellProjection, SkinSnapshot,
};
use dnatreecalc_host::app::{
    DnaTreeWorkspaceDocument, HostSessionExecutor, SessionExecutor, TreeWorkspaceSession,
};
use dnatreecalc_worker::{WorkerInbound, WorkerOutbound};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

struct WorkerState {
    // The reactive owner the executor's signals belong to (kept alive).
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

/// Install the message handler. Called from `start()` when it detects it is
/// running in a worker.
pub fn start_worker() {
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
        Ok(WorkerInbound::Intent { envelope }) => {
            STATE.with(|state| match state.borrow().as_ref() {
                Some(state) => post(&WorkerOutbound::Response {
                    response: Box::new(state.executor.execute(envelope)),
                }),
                None => post(&WorkerOutbound::Failed {
                    message: "intent arrived before the worker was initialized".to_string(),
                }),
            })
        }
        Ok(WorkerInbound::SharedIntent { envelope }) => {
            let intent_id = envelope.intent_id.clone();
            if let Err(error) = envelope.validate() {
                post(&WorkerOutbound::SharedResponse {
                    receipt: Box::new(SkinIntentReceipt::Rejected {
                        intent_id,
                        diagnostic: SkinIntentDiagnostic {
                            code: "skin_protocol_invalid".into(),
                            message: format!("{error:?}"),
                            recoverable: true,
                        },
                    }),
                    snapshot: None,
                });
                return;
            }
            let SkinIntent::TreeWorkspace(intent) = envelope.intent else {
                post(&WorkerOutbound::SharedResponse {
                    receipt: Box::new(SkinIntentReceipt::Rejected {
                        intent_id,
                        diagnostic: SkinIntentDiagnostic {
                            code: "unsupported_intent_family".into(),
                            message: "worker session accepts tree workspace intents".into(),
                            recoverable: true,
                        },
                    }),
                    snapshot: None,
                });
                return;
            };
            STATE.with(|state| match state.borrow().as_ref() {
                Some(state) => {
                    let response = state.executor.execute(IntentEnvelope { seq: 0, intent });
                    let workspace = state.executor.snapshot();
                    let revision = workspace.projection_seq;
                    let receipt = if response.receipt.accepted {
                        SkinIntentReceipt::Applied {
                            intent_id,
                            snapshot_revision: revision,
                        }
                    } else {
                        SkinIntentReceipt::Rejected {
                            intent_id,
                            diagnostic: SkinIntentDiagnostic {
                                code: "workspace_intent_rejected".into(),
                                message: response.receipt.error.map_or_else(
                                    || "intent rejected".into(),
                                    |error| error.to_string(),
                                ),
                                recoverable: true,
                            },
                        }
                    };
                    let snapshot = Some(Box::new(SkinSnapshot::tree_workspace(
                        SkinShellProjection {
                            host_kind: HostKindProjection::TreeCalc,
                            title: "DNA TreeCalc".into(),
                            active_document_id: envelope.document_id,
                            status_text: None,
                            command_palette_open: false,
                            persistence: Default::default(),
                        },
                        HostCapabilityProjection::treecalc_references(
                            RuntimeProfileProjection::BrowserWasm,
                            ExtensionPlacementProjection::Unavailable,
                        ),
                        workspace,
                    )));
                    post(&WorkerOutbound::SharedResponse {
                        receipt: Box::new(receipt),
                        snapshot,
                    });
                }
                None => post(&WorkerOutbound::Failed {
                    message: "intent arrived before the worker was initialized".into(),
                }),
            })
        }
        Err(error) => post(&WorkerOutbound::Failed {
            message: format!("could not decode a worker message: {error}"),
        }),
    }
}

fn init_session(document: DnaTreeWorkspaceDocument) {
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
            let shared_snapshot = SkinSnapshot::tree_workspace(
                SkinShellProjection {
                    host_kind: HostKindProjection::TreeCalc,
                    title: "DNA TreeCalc".into(),
                    active_document_id: None,
                    status_text: None,
                    command_palette_open: false,
                    persistence: Default::default(),
                },
                HostCapabilityProjection::treecalc_references(
                    RuntimeProfileProjection::BrowserWasm,
                    ExtensionPlacementProjection::Unavailable,
                ),
                STATE.with(|state| {
                    state
                        .borrow()
                        .as_ref()
                        .expect("stored worker state")
                        .executor
                        .snapshot()
                }),
            );
            post(&WorkerOutbound::SharedReady {
                snapshot: Box::new(shared_snapshot),
            });
        }
        Err(message) => post(&WorkerOutbound::Failed { message }),
    }
}

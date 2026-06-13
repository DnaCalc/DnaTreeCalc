//! Main-thread glue for the off-thread calculation worker (live B.2.2).
//!
//! [`WebWorkerDispatcher`] is a [`Dispatcher`] that fronts a `web_sys::Worker`
//! running the session. View intents (selection, persona) are answered on the
//! main thread; engine intents go through [`WorkerProxyCore`] and out over
//! `postMessage`, returning an accepted-and-pending receipt immediately. The
//! worker's responses arrive on `onmessage`, where [`WorkerProxyCore::deliver`]
//! applies the snapshot/delta to the mirror signals and the returned selection.
//!
//! The `web_sys::Worker`, the closure, and the proxy are `!Send`, so they live
//! in a thread-local keyed by a runtime id (the `HOST_SESSIONS` pattern); the
//! dispatcher itself holds only the `Send + Sync` signals + shared handle.
//!
//! This is an **opt-in** path (`?worker=1`); the default app keeps the
//! synchronous in-process dispatcher. First-cut simplifications (documented):
//! no live preview (the session is off-thread — lenses degrade to post-attempt
//! receipts), no main-thread persona gate or document persistence in worker
//! mode, and `SelectNodes` is mirrored without host-side key validation.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use dnatreecalc_host::app::WorkerProxyCore;
use dnatreecalc_skin_framework::{
    Dispatcher, IntentError, IntentReceipt, SelectionState, SharedSkinStateHandle,
    SharedStateChange, SharedStateOrigin, TableCellSelection, WorkspaceDelta, WorkspaceIntent,
    WorkspaceState,
};
use dnatreecalc_worker::{WorkerInbound, WorkerOutbound};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Blob, BlobPropertyBag, MessageEvent, Url, Worker, WorkerOptions, WorkerType};

const WORKER_BOOTSTRAPPED: &str = "__dnatreecalc_worker_bootstrapped__";

/// The module-worker bootstrap: import the same Trunk-emitted ES module that
/// the page uses and initialize it with the matching hashed wasm URL.
/// wasm-bindgen runs `start()` after init, which detects the worker context and
/// installs the message handler.
fn worker_bootstrap_js(module_url: &str, wasm_url: &str) -> String {
    format!(
        "try {{\n\
           const {{ default: init }} = await import({module_url:?});\n\
           await init({{ module_or_path: {wasm_url:?} }});\n\
           globalThis.postMessage({WORKER_BOOTSTRAPPED:?});\n\
         }} catch (error) {{\n\
           console.error('dnatreecalc worker bootstrap failed', error);\n\
           throw error;\n\
         }}\n"
    )
}

fn app_module_url(window: &web_sys::Window) -> Result<String, JsValue> {
    let page_url = window.location().href()?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    if let Some(modulepreload) =
        document.query_selector("link[rel=\"modulepreload\"][href$=\".js\"]")?
        && let Some(href) = modulepreload.get_attribute("href")
    {
        return Ok(Url::new_with_base(&href, &page_url)?.href());
    }
    Ok(Url::new_with_base("dnatreecalc-web.js", &page_url)?.href())
}

fn wasm_url_for_module(module_url: &str) -> Result<String, JsValue> {
    module_url
        .strip_suffix(".js")
        .map(|stem| format!("{stem}_bg.wasm"))
        .ok_or_else(|| JsValue::from_str("worker module URL did not end in .js"))
}

struct WorkerRuntime {
    worker: Worker,
    proxy: WorkerProxyCore,
    pending_init: Option<String>,
    // Kept alive for the worker's lifetime (dropping it detaches onmessage).
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    workspace: RwSignal<WorkspaceState>,
    latest_delta: RwSignal<WorkspaceDelta>,
    selection: RwSignal<SelectionState>,
}

thread_local! {
    static RUNTIMES: RefCell<BTreeMap<u64, WorkerRuntime>> = const { RefCell::new(BTreeMap::new()) };
}

static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(1);

/// A [`Dispatcher`] backed by an off-thread `web_sys::Worker`.
pub struct WebWorkerDispatcher {
    runtime_id: u64,
    selection: RwSignal<SelectionState>,
    shared: SharedSkinStateHandle,
}

impl WebWorkerDispatcher {
    /// Spawn the worker at `script_url`, send it the initial document, and
    /// return a dispatcher over it. `initial` seeds the proxy mirror (and the
    /// `workspace` signal already holds it). Errors if the `Worker` cannot be
    /// constructed (the caller falls back to in-process dispatch).
    pub fn new(
        initial: WorkspaceState,
        document: dnatreecalc_host::app::DnaTreeWorkspaceDocument,
        workspace: RwSignal<WorkspaceState>,
        latest_delta: RwSignal<WorkspaceDelta>,
        selection: RwSignal<SelectionState>,
        shared: SharedSkinStateHandle,
    ) -> Result<Self, JsValue> {
        let runtime_id = NEXT_RUNTIME_ID.fetch_add(1, Ordering::Relaxed);

        // Spawn a module worker from a blob bootstrap that imports the same
        // generated module as the page. The worker gets its own wasm instance;
        // inside that context `start()` installs the worker message loop.
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
        let module_url = app_module_url(&window)?;
        let wasm_url = wasm_url_for_module(&module_url)?;
        let bootstrap_js = worker_bootstrap_js(&module_url, &wasm_url);
        let parts = js_sys::Array::new();
        parts.push(&JsValue::from_str(&bootstrap_js));
        let bag = BlobPropertyBag::new();
        bag.set_type("text/javascript");
        let blob = Blob::new_with_str_sequence_and_options(&parts, &bag)?;
        let url = Url::create_object_url_with_blob(&blob)?;
        // Not revoked: the worker may still be fetching its script from the
        // blob URL; one leaked object URL per session is negligible.
        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);
        let worker = Worker::new_with_options(&url, &options)?;

        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            on_worker_message(runtime_id, event);
        });
        worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        // Hand the worker its initial document after the bootstrap tells us
        // wasm-bindgen `start()` installed the worker's `onmessage` handler.
        let init = WorkerInbound::Init {
            document: Box::new(document),
        };
        let pending_init =
            serde_json::to_string(&init).map_err(|error| JsValue::from_str(&error.to_string()))?;

        RUNTIMES.with(|runtimes| {
            runtimes.borrow_mut().insert(
                runtime_id,
                WorkerRuntime {
                    worker,
                    proxy: WorkerProxyCore::new(initial),
                    pending_init: Some(pending_init),
                    _on_message: on_message,
                    workspace,
                    latest_delta,
                    selection,
                },
            );
        });

        Ok(Self {
            runtime_id,
            selection,
            shared,
        })
    }
}

impl Dispatcher for WebWorkerDispatcher {
    fn dispatch(&self, intent: WorkspaceIntent) -> IntentReceipt {
        use WorkspaceIntent as I;
        match intent {
            // Pure view intents are answered on the main thread — no round trip,
            // instant even while the worker is busy recalculating.
            I::SelectNode(node) => {
                self.selection.set(SelectionState::with_primary(node));
                IntentReceipt::accepted()
            }
            I::SelectTableCell {
                table,
                row_id,
                column_id,
            } => {
                self.selection
                    .set(SelectionState::with_table_cell(TableCellSelection {
                        table,
                        row_id,
                        column_id,
                    }));
                IntentReceipt::accepted()
            }
            I::SelectNodes { keys, anchor } => {
                self.shared.apply(
                    SharedStateChange::SetSelectionSet(keys),
                    SharedStateOrigin::Host,
                );
                self.shared.apply(
                    SharedStateChange::SetSelectionAnchor(anchor),
                    SharedStateOrigin::Host,
                );
                IntentReceipt::accepted()
            }
            I::SetPersona { persona } => {
                self.shared.apply(
                    SharedStateChange::SetPersona(persona),
                    SharedStateOrigin::Host,
                );
                IntentReceipt::accepted()
            }
            // Everything else runs on the engine, off-thread.
            engine_intent => RUNTIMES.with(|runtimes| {
                let mut runtimes = runtimes.borrow_mut();
                let Some(runtime) = runtimes.get_mut(&self.runtime_id) else {
                    return IntentReceipt::rejected(IntentError::Unsupported);
                };
                let decision = runtime.proxy.submit(engine_intent);
                if let Some(envelope) = decision.to_send {
                    post_intent(&runtime.worker, envelope);
                }
                decision.receipt
            }),
        }
    }
}

/// Serialize and `postMessage` an intent envelope to the worker.
fn post_intent(worker: &Worker, envelope: dnatreecalc_skin_framework::IntentEnvelope) {
    if let Ok(json) = serde_json::to_string(&WorkerInbound::Intent { envelope }) {
        let _ = worker.post_message(&JsValue::from_str(&json));
    }
}

/// Handle one message from the worker: apply a response to the mirror signals,
/// or seed the initial state, or log a failure.
fn on_worker_message(runtime_id: u64, event: MessageEvent) {
    let Some(text) = event.data().as_string() else {
        return;
    };
    if text == WORKER_BOOTSTRAPPED {
        RUNTIMES.with(|runtimes| {
            let mut runtimes = runtimes.borrow_mut();
            let Some(runtime) = runtimes.get_mut(&runtime_id) else {
                return;
            };
            if let Some(init) = runtime.pending_init.take() {
                let _ = runtime.worker.post_message(&JsValue::from_str(&init));
            }
        });
        return;
    }
    let Ok(outbound) = serde_json::from_str::<WorkerOutbound>(&text) else {
        return;
    };
    RUNTIMES.with(|runtimes| {
        let mut runtimes = runtimes.borrow_mut();
        let Some(runtime) = runtimes.get_mut(&runtime_id) else {
            return;
        };
        match outbound {
            WorkerOutbound::Ready { snapshot, .. } => {
                runtime.workspace.set(*snapshot);
            }
            WorkerOutbound::Response { response } => {
                let delta = response.receipt.delta.clone();
                let outcome = runtime.proxy.deliver(*response, 0);
                if outcome.applied {
                    runtime.workspace.set(runtime.proxy.mirror().clone());
                    runtime.latest_delta.set(delta);
                    if let Some(selection) = outcome.selection {
                        runtime.selection.set(selection);
                    }
                }
                // A run completed; release the next parked intent, if any.
                if let Some(envelope) = outcome.next {
                    post_intent(&runtime.worker, envelope);
                }
            }
            WorkerOutbound::Failed { message } => {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "dnatreecalc worker: {message}"
                )));
            }
        }
    });
}

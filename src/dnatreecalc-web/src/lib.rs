//! DNA TreeCalc WASM entry point.
//!
//! Mounts the walking-skeleton shell into a `<div id="…">` host element
//! from the browser side. Native (non-wasm) targets compile the same
//! crate as an `rlib` so workspace-wide `cargo check`/`cargo test`
//! cover both code paths; the `mount_dnatreecalc` entry only exists on
//! `wasm32`. The shell, framework, and skin crates do not depend on
//! `web-sys` or `wasm-bindgen` — that coupling lives only here.

#[cfg(target_arch = "wasm32")]
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use dnatreecalc_host::app::{
    DnaTreeWorkspaceDocument, HostDispatcher, WorkbookHostDispatcher, WorkspaceDocumentCatalog,
    WorkspaceDocumentStore, WorkspaceDocumentStoreError, build_default_registry,
    workspace_session_from_document_store_or_default,
};
#[cfg(target_arch = "wasm32")]
use dnatreecalc_shell::WorkspaceShell;
#[cfg(target_arch = "wasm32")]
use dnatreecalc_skin_framework::{
    Dispatcher, NodeId, PersistedSkinStateRecord, SelectionState, SharedSkinState,
    SharedSkinStateHandle, SkinStatePersistenceError, SkinStatePersistenceKey,
    SkinStatePersistenceStore, ThemeTokens, WorkspaceDelta, WorkspaceState,
};
#[cfg(target_arch = "wasm32")]
use dnatreecalc_skins::{FLOW_ID, WORKBOOK_ID};
#[cfg(target_arch = "wasm32")]
use leptos::mount::mount_to;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
mod worker_client;
#[cfg(target_arch = "wasm32")]
mod worker_runtime;

#[cfg(target_arch = "wasm32")]
fn install_wasm_diagnostics() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "dnatreecalc wasm panic: {info}"
        )));
    }));
}

#[cfg(target_arch = "wasm32")]
struct BrowserLocalStorageSkinStateStore {
    storage: web_sys::Storage,
    prefix: String,
}

#[cfg(target_arch = "wasm32")]
struct BrowserLocalStorageWorkspaceDocumentStore {
    storage: web_sys::Storage,
    catalog_key: String,
    workspace_prefix: String,
}

#[cfg(target_arch = "wasm32")]
impl BrowserLocalStorageWorkspaceDocumentStore {
    fn new(storage: web_sys::Storage) -> Self {
        Self {
            storage,
            catalog_key: "dnatreecalc:workspace-documents:catalog".to_string(),
            workspace_prefix: "dnatreecalc:workspace-documents:workspace:".to_string(),
        }
    }

    fn workspace_key(&self, workspace_id: &str) -> String {
        format!("{}{}", self.workspace_prefix, workspace_id)
    }
}

#[cfg(target_arch = "wasm32")]
impl WorkspaceDocumentStore for BrowserLocalStorageWorkspaceDocumentStore {
    fn load_catalog(
        &self,
    ) -> Result<Option<WorkspaceDocumentCatalog>, WorkspaceDocumentStoreError> {
        let Some(text) = self.storage.get_item(&self.catalog_key).map_err(|error| {
            WorkspaceDocumentStoreError::Store {
                operation: "reading browser workspace catalog",
                detail: format!("{error:?}"),
            }
        })?
        else {
            return Ok(None);
        };
        serde_json::from_str(&text)
            .map(Some)
            .map_err(|error| WorkspaceDocumentStoreError::Deserialize(error.to_string()))
    }

    fn save_catalog(
        &self,
        catalog: &WorkspaceDocumentCatalog,
    ) -> Result<(), WorkspaceDocumentStoreError> {
        let text = serde_json::to_string(catalog)
            .map_err(|error| WorkspaceDocumentStoreError::Serialize(error.to_string()))?;
        self.storage
            .set_item(&self.catalog_key, &text)
            .map_err(|error| WorkspaceDocumentStoreError::Store {
                operation: "writing browser workspace catalog",
                detail: format!("{error:?}"),
            })
    }

    fn load_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Option<DnaTreeWorkspaceDocument>, WorkspaceDocumentStoreError> {
        let key = self.workspace_key(workspace_id);
        let Some(text) =
            self.storage
                .get_item(&key)
                .map_err(|error| WorkspaceDocumentStoreError::Store {
                    operation: "reading browser workspace document",
                    detail: format!("{error:?}"),
                })?
        else {
            return Ok(None);
        };
        serde_json::from_str(&text)
            .map(Some)
            .map_err(|error| WorkspaceDocumentStoreError::Deserialize(error.to_string()))
    }

    fn save_workspace(
        &self,
        workspace_id: &str,
        document: &DnaTreeWorkspaceDocument,
    ) -> Result<(), WorkspaceDocumentStoreError> {
        let text = serde_json::to_string(document)
            .map_err(|error| WorkspaceDocumentStoreError::Serialize(error.to_string()))?;
        self.storage
            .set_item(&self.workspace_key(workspace_id), &text)
            .map_err(|error| WorkspaceDocumentStoreError::Store {
                operation: "writing browser workspace document",
                detail: format!("{error:?}"),
            })
    }
}

#[cfg(target_arch = "wasm32")]
impl BrowserLocalStorageSkinStateStore {
    fn new(storage: web_sys::Storage) -> Self {
        Self {
            storage,
            prefix: "dnatreecalc:skin-state:".to_string(),
        }
    }

    fn key_for(&self, key: &SkinStatePersistenceKey) -> String {
        format!("{}{}", self.prefix, key.storage_key())
    }
}

#[cfg(target_arch = "wasm32")]
impl SkinStatePersistenceStore for BrowserLocalStorageSkinStateStore {
    fn load(
        &self,
        key: &SkinStatePersistenceKey,
    ) -> Result<Option<PersistedSkinStateRecord>, SkinStatePersistenceError> {
        let storage_key = self.key_for(key);
        let Some(text) = self.storage.get_item(&storage_key).map_err(|error| {
            SkinStatePersistenceError::Store {
                operation: "reading browser localStorage",
                detail: format!("{error:?}"),
            }
        })?
        else {
            return Ok(None);
        };
        serde_json::from_str(&text)
            .map(Some)
            .map_err(|error| SkinStatePersistenceError::Deserialize(error.to_string()))
    }

    fn save(
        &self,
        key: &SkinStatePersistenceKey,
        record: &PersistedSkinStateRecord,
    ) -> Result<(), SkinStatePersistenceError> {
        let text = serde_json::to_string(record)
            .map_err(|error| SkinStatePersistenceError::Serialize(error.to_string()))?;
        self.storage
            .set_item(&self.key_for(key), &text)
            .map_err(|error| SkinStatePersistenceError::Store {
                operation: "writing browser localStorage",
                detail: format!("{error:?}"),
            })
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn mount_dnatreecalc(element_id: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let host_element = document
        .get_element_by_id(element_id)
        .ok_or_else(|| JsValue::from_str("mount element not found"))?
        .dyn_into::<web_sys::HtmlElement>()?;

    let local_storage = window
        .local_storage()?
        .ok_or_else(|| JsValue::from_str("localStorage unavailable"))?;

    // Opt-in (`?wb=1`): mount a strict-Excel workbook driven through the clean
    // `dnacalc-host-core` spine (Phase 0 keystone) — a two-sheet demo workbook
    // with live formulas, viewable and editable in BOTH the Notebook and the
    // Workbook (sheet-mode) lenses. Distinct from `?grid=1`, which attaches a
    // demo grid to the legacy tree session. Early-returns its own mount.
    let workbook_demo = window
        .location()
        .search()
        .map(|search| search.contains("wb=1"))
        .unwrap_or(false);
    if workbook_demo {
        let workspace = RwSignal::new(WorkspaceState::default());
        let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(0));
        let selection = RwSignal::new(SelectionState::with_primary(None));
        let shared = SharedSkinStateHandle::new(SharedSkinState::default());
        let skin_state_store: Arc<dyn SkinStatePersistenceStore> = Arc::new(
            BrowserLocalStorageSkinStateStore::new(local_storage.clone()),
        );
        let dispatcher = Arc::new(
            WorkbookHostDispatcher::new_demo(workspace, latest_delta, selection, Some(shared))
                .map_err(|error| JsValue::from_str(&error.to_string()))?,
        );
        web_sys::console::log_1(
            &"?wb=1 host-core workbook demo mounted -- switch between the Notebook and Workbook lenses"
                .into(),
        );
        let dispatch: Arc<dyn Dispatcher> = dispatcher;
        let preview: Option<Arc<dyn dnatreecalc_skin_framework::PreviewService>> = None;
        let registry = Arc::new(build_default_registry());
        let mount_handle = mount_to(host_element.clone(), move || {
            view! {
                <WorkspaceShell
                    workspace=workspace.read_only()
                    latest_delta=latest_delta.read_only()
                    selection=selection
                    shared=shared
                    skin_state_store=skin_state_store.clone()
                    dispatch=dispatch.clone()
                    registry=registry.clone()
                    initial_skin=WORKBOOK_ID
                    tokens=ThemeTokens::light()
                    preview=preview.clone()
                />
            }
        });
        std::mem::forget(mount_handle);
        return Ok(());
    }

    let workspace_document_store: Arc<dyn WorkspaceDocumentStore> = Arc::new(
        BrowserLocalStorageWorkspaceDocumentStore::new(local_storage.clone()),
    );
    let (mut session, restored_selection) =
        workspace_session_from_document_store_or_default(workspace_document_store.as_ref())
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
    // The real restored selection (before the dev fallback below) is what the
    // worker should reopen with.
    let selected_for_document = restored_selection.clone();
    // Dev affordance (`?grid=1`): attach a demonstration grid to the workspace
    // root so the Sheet lens (Ctrl+4) renders a scrollable grid surface; scrolling
    // re-scopes the window via SetGridInterest. Off by default.
    let grid_demo = window
        .location()
        .search()
        .map(|search| search.contains("grid=1"))
        .unwrap_or(false);
    let grid_demo_node = if grid_demo {
        match session.attach_demo_grid() {
            Ok(node) => {
                web_sys::console::log_1(
                    &format!(
                        "?grid=1 demo grid attached to node {node} -- open the Sheet lens (Ctrl+4)"
                    )
                    .into(),
                );
                Some(node)
            }
            Err(error) => {
                web_sys::console::error_1(&format!("?grid=1 demo grid failed: {error}").into());
                None
            }
        }
    } else {
        None
    };
    let session = Arc::new(std::sync::Mutex::new(session));
    let workspace_state = session
        .lock()
        .map_err(|_| JsValue::from_str("workspace session mutex poisoned"))?
        .workspace_state()
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let workspace = RwSignal::new(workspace_state);
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(
        workspace.get_untracked().projection_seq,
    ));
    let selection = RwSignal::new(SelectionState::with_primary(
        grid_demo_node
            .clone()
            .or(restored_selection)
            .or_else(|| Some(NodeId::new("Sheet1.RandArray5x5"))),
    ));
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());
    let skin_state_store: Arc<dyn SkinStatePersistenceStore> =
        Arc::new(BrowserLocalStorageSkinStateStore::new(local_storage));

    // Opt-in (`?worker=1`): run the session in a web worker. The default path
    // keeps the synchronous in-process dispatcher (and its live preview).
    let worker_mode = window
        .location()
        .search()
        .map(|search| search.contains("worker=1"))
        .unwrap_or(false);

    let (dispatch, preview): (
        Arc<dyn Dispatcher>,
        Option<Arc<dyn dnatreecalc_skin_framework::PreviewService>>,
    ) = if worker_mode {
        let document = session
            .lock()
            .map_err(|_| JsValue::from_str("workspace session mutex poisoned"))?
            .export_dnatree_document(selected_for_document.as_ref())
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        let initial = workspace.get_untracked();
        let dispatcher = worker_client::WebWorkerDispatcher::new(
            initial,
            document,
            workspace,
            latest_delta,
            selection,
            shared,
        )?;
        // Preview reaches the session synchronously; off-thread it is
        // unavailable, so lenses fall back to post-attempt receipts.
        (Arc::new(dispatcher), None)
    } else {
        let dispatcher = Arc::new(HostDispatcher::with_session_shared_and_workspace_store(
            selection,
            workspace,
            latest_delta,
            session,
            Some(shared),
            Some(workspace_document_store),
        ));
        let dispatch: Arc<dyn Dispatcher> = dispatcher.clone();
        // The live dispatcher also answers non-mutating previews (tenet 7).
        let preview: Arc<dyn dnatreecalc_skin_framework::PreviewService> = dispatcher;
        (dispatch, Some(preview))
    };

    let registry = Arc::new(build_default_registry());

    let mount_handle = mount_to(host_element, move || {
        view! {
            <WorkspaceShell
                workspace=workspace.read_only()
                latest_delta=latest_delta.read_only()
                selection=selection
                shared=shared
                skin_state_store=skin_state_store.clone()
                dispatch=dispatch.clone()
                registry=registry.clone()
                initial_skin=FLOW_ID
                tokens=ThemeTokens::light()
                preview=preview.clone()
            />
        }
    });
    std::mem::forget(mount_handle);
    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    install_wasm_diagnostics();
    // This crate is built twice: as the main app (`--target web`) and as the
    // calculation worker. A worker has no `window`, so that is how we tell
    // which context is running.
    if web_sys::window().is_some() {
        web_sys::console::log_1(&JsValue::from_str("dnatreecalc wasm start: window"));
        mount_dnatreecalc("dnatreecalc-app")
    } else {
        web_sys::console::log_1(&JsValue::from_str("dnatreecalc wasm start: worker"));
        worker_runtime::start_worker();
        Ok(())
    }
}

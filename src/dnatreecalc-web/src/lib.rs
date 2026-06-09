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
    HostDispatcher, build_default_registry, preview_accounts_workspace_session,
};
#[cfg(target_arch = "wasm32")]
use dnatreecalc_shell::WorkspaceShell;
#[cfg(target_arch = "wasm32")]
use dnatreecalc_skin_framework::{
    Dispatcher, NodeId, PersistedSkinStateRecord, SelectionState, SharedSkinState,
    SharedSkinStateHandle, SkinStatePersistenceError, SkinStatePersistenceKey,
    SkinStatePersistenceStore, ThemeTokens, WorkspaceDelta,
};
#[cfg(target_arch = "wasm32")]
use dnatreecalc_skins::TRIPLE_EDITOR_ID;
#[cfg(target_arch = "wasm32")]
use leptos::mount::mount_to;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
struct BrowserLocalStorageSkinStateStore {
    storage: web_sys::Storage,
    prefix: String,
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

    let session = Arc::new(std::sync::Mutex::new(preview_accounts_workspace_session()));
    let workspace_state = session
        .lock()
        .map_err(|_| JsValue::from_str("workspace session mutex poisoned"))?
        .workspace_state()
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let workspace = RwSignal::new(workspace_state);
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(
        workspace.get_untracked().projection_seq,
    ));
    let selection = RwSignal::new(SelectionState::with_primary(Some(NodeId::new(
        "Sheet1.RandArray5x5",
    ))));
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());
    let skin_state_store: Arc<dyn SkinStatePersistenceStore> =
        Arc::new(BrowserLocalStorageSkinStateStore::new(
            window
                .local_storage()?
                .ok_or_else(|| JsValue::from_str("localStorage unavailable"))?,
        ));

    let dispatcher = Arc::new(HostDispatcher::with_session_and_shared(
        selection,
        workspace,
        latest_delta,
        session,
        Some(shared),
    ));
    let dispatch: Arc<dyn Dispatcher> = dispatcher;

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
                initial_skin=TRIPLE_EDITOR_ID
                tokens=ThemeTokens::light()
            />
        }
    });
    std::mem::forget(mount_handle);
    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    mount_dnatreecalc("dnatreecalc-app")
}

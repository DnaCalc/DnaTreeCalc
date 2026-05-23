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
    HostDispatcher, build_default_registry, preview_accounts_workspace_state,
};
#[cfg(target_arch = "wasm32")]
use dnatreecalc_shell::WorkspaceShell;
#[cfg(target_arch = "wasm32")]
use dnatreecalc_skin_framework::{
    Dispatcher, SelectionState, SharedSkinState, SharedSkinStateHandle,
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

    let workspace_state = preview_accounts_workspace_state();
    let workspace = RwSignal::new(workspace_state);
    let selection = RwSignal::new(SelectionState::default());
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());

    let dispatcher = Arc::new(HostDispatcher::new(selection, None));
    let dispatch: Arc<dyn Dispatcher> = dispatcher;

    let registry = Arc::new(build_default_registry());

    let mount_handle = mount_to(host_element, move || {
        view! {
            <WorkspaceShell
                workspace=workspace.read_only()
                selection=selection
                shared=shared
                dispatch=dispatch.clone()
                registry=registry.clone()
                initial_skin=TRIPLE_EDITOR_ID
            />
        }
    });
    std::mem::forget(mount_handle);
    Ok(())
}

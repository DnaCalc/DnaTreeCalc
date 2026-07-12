#![recursion_limit = "512"]

#[cfg(all(target_arch = "wasm32", feature = "shell-ui"))]
use wasm_bindgen::prelude::*;

pub mod adapters;
pub mod app;
pub mod domain;
pub mod extensions;
pub mod persistence;
pub mod platform;
pub mod services;
pub mod state;
pub mod test_support;
pub mod ui;

#[cfg(all(target_arch = "wasm32", feature = "shell-ui"))]
#[wasm_bindgen]
pub fn mount_onecalc_preview(element_id: &str) -> Result<(), JsValue> {
    use leptos::mount::mount_to;
    use leptos::prelude::*;
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let host = document
        .get_element_by_id(element_id)
        .ok_or_else(|| JsValue::from_str("preview mount element not found"))?
        .dyn_into::<web_sys::HtmlElement>()?;

    // WS-14 Pre-MVP P7: mount the new minimal HomeShell instead of the legacy
    // three-mode OneCalcShellApp. The legacy shell + mode shells stay compiled
    // in tree until the WS-14 cleanup phase retires them.
    let initial_state = app::preview_state::preview_minimal_host_state();
    let editor_bridge =
        app::host_mount::bootstrap_editor_bridge(app::host_mount::HostMountTarget::WebBrowser);
    let mount_handle = mount_to(host, move || {
        view! {
            <ui::components::home_shell::HomeShell
                initial_state=initial_state.clone()
                editor_bridge=Some(editor_bridge.clone())
            />
        }
    });
    std::mem::forget(mount_handle);
    Ok(())
}

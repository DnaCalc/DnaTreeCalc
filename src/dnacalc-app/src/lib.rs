//! dnacalc-app — the DNA Calc product skeleton (dtc-tsc.9,
//! SHELL_SPEC §10 exit acceptance).
//!
//! TIER: Calc app root (TC-adjacent). Composes the TP shell + bridge, but its
//! dispatcher drives a real `dnacalc-host-core` workbook (oxcalc IS in this
//! graph, by design). NOT a TP crate; NOT in `dnacalc-arch-gates::TP_CRATES`.
//! See [`app::CalcApp`] for the composition and [`adapter`] for the DEGRADE →
//! `EnterGridCell` seam with the three-way outcome.
//!
//! Targets: the `cdylib` is mounted in the browser by [`start`] (trunk); native
//! `cargo test` compiles the `rlib` and exercises [`adapter`] against the real
//! host-core engine. The browser DOM half lives in `tests/browser.rs`.

pub mod adapter;
pub mod app;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Mount the Calc app into the element with `element_id` (browser only).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn mount_calc(element_id: &str) -> Result<(), JsValue> {
    use dnacalc_shell::RuntimeContext;
    use leptos::mount::mount_to;
    use leptos::prelude::*;
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let host = document
        .get_element_by_id(element_id)
        .ok_or_else(|| JsValue::from_str("mount element not found"))?
        .dyn_into::<web_sys::HtmlElement>()?;

    let mount_handle = mount_to(host, move || {
        view! { <app::CalcApp runtime=RuntimeContext::Browser /> }
    });
    std::mem::forget(mount_handle);
    Ok(())
}

/// Trunk/WASM entry point.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&JsValue::from_str(&format!("dnacalc-app panic: {info}")));
    }));
    mount_calc("dnacalc-app")
}

//! dnacalc-bench-app — the DNA Bench product skeleton (dtc-tsc.9,
//! SHELL_SPEC §10 exit acceptance; BENCH_SPEC §2 layout).
//!
//! TIER: TF. Composes the TP shell (`dnacalc-shell`) + workbench
//! (`dnacalc-bridge`, FULL mode) over the real `dnacalc-bench-host` OneFormula
//! host (oxfml + oxfunc; the F-gate keeps oxcalc out). See [`app::BenchApp`]
//! for the composition and [`adapter::BenchHost`] for the bridge-event → intent
//! seam the bridge itself never crosses.
//!
//! Targets: the `cdylib` is mounted in the browser by [`start`] (trunk);
//! native `cargo test` compiles the `rlib` and exercises [`adapter`] without a
//! browser. The browser DOM half lives in `tests/browser.rs`.

pub mod adapter;
pub mod app;
pub mod extensions;
pub mod format_panel;
pub mod xray;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Mount the Bench app into the element with `element_id` (browser only).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn mount_bench(element_id: &str) -> Result<(), JsValue> {
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
        view! { <app::BenchApp runtime=RuntimeContext::Browser /> }
    });
    std::mem::forget(mount_handle);
    Ok(())
}

/// Trunk/WASM entry point.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "dnacalc-bench-app panic: {info}"
        )));
    }));
    mount_bench("dnacalc-bench-app")
}

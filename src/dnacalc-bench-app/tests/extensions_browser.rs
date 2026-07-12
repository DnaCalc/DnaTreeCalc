//! Browser suite for the Bench Extensions manager v0 (BENCH_SPEC §6,
//! mechanism 18, bead dtc-lfz.6).
//!
//! Mounts the real [`BenchApp`] over the real OneFormula host in a live
//! headless browser (a genuine `wasm32` compile — `cfg!(target_arch =
//! "wasm32")` is true, so the host's `HostCapabilityProjection.runtime_profile`
//! resolves to `BrowserWasm` for real, not by override). Proves:
//! - the Strip's Feeds instrument is the documented entry point (BENCH_SPEC
//!   §2/§6: "reached from Strip/Inspector") and opens the overlay;
//! - the overlay renders the runtime-honest BrowserWasm state (native
//!   providers explicitly unavailable, "requires desktop or a companion
//!   process") — never a fake OK;
//! - honest absence: with no live provider catalog anywhere in this product
//!   (see `dnacalc_bench_app::extensions` module doc), the provider table
//!   renders "No providers are registered", never a fabricated row;
//! - Esc closes the overlay through the shell's ordinary one-at-a-time
//!   overlay ladder (no bespoke close logic).
//!
//! Every assertion is a behaviour the pre-fix build did not have (there was
//! no Feeds click handler, no Extensions overlay slot, no honest-runtime
//! banner), so the suite fails against pre-fix code — verified by stashing
//! `src/dnacalc-bench-app/src/extensions.rs` plus the shell/app wiring.

#![cfg(target_arch = "wasm32")]

use dnacalc_bench_app::app::BenchApp;
use dnacalc_shell::RuntimeContext;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

async fn next_tick() {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        web_sys::window()
            .expect("window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0)
            .expect("setTimeout");
    });
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .expect("tick");
}

async fn settle() {
    next_tick().await;
    next_tick().await;
}

fn mount() -> web_sys::HtmlElement {
    let document = web_sys::window().unwrap().document().unwrap();
    let host = document
        .create_element("div")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    document.body().unwrap().append_child(&host).unwrap();
    leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        view! { <BenchApp runtime=RuntimeContext::Browser /> }
    })
    .forget();
    host
}

fn query(host: &web_sys::HtmlElement, selector: &str) -> Option<web_sys::Element> {
    host.query_selector(selector).unwrap()
}

fn click(element: &web_sys::Element) {
    let event = web_sys::MouseEvent::new("click").unwrap();
    element.dispatch_event(&event).unwrap();
}

fn press_chord(target: &web_sys::EventTarget, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    target.dispatch_event(&event).unwrap();
}

/// BENCH_SPEC §2/§6: the Extensions manager is "an overlay reached from
/// Strip/Inspector" — the Strip's Feeds instrument (mechanism 18) is that
/// entry point, and it is wired (`data-clickable="true"`) once
/// `host_capabilities` is real (bead dtc-lfz.6 wires it in `BenchApp`).
#[wasm_bindgen_test]
async fn feeds_strip_instrument_is_wired_and_opens_the_extensions_overlay() {
    let host = mount();
    settle().await;

    let feeds = query(&host, "[data-slot=\"feeds\"]").expect("Feeds strip slot renders");
    assert_eq!(
        feeds.get_attribute("data-clickable").as_deref(),
        Some("true"),
        "Feeds is the documented Strip entry point once host_capabilities is wired"
    );
    // Real, honest macro-state text — not the old reserved-nothing slot.
    assert!(
        feeds.text_content().unwrap_or_default().starts_with("ext:"),
        "Feeds shows the honest ext: <placement> readout"
    );

    assert!(
        query(&host, "[data-overlay=\"extensions\"]").is_none(),
        "the overlay is closed until Feeds is clicked"
    );

    click(&feeds);
    settle().await;

    assert!(
        query(&host, "[data-overlay=\"extensions\"]").is_some(),
        "clicking Feeds opens the Extensions overlay"
    );
}

/// BENCH_SPEC §6: per-runtime honesty is a lookup, not skin logic — a real
/// `wasm32` browser compile resolves `RuntimeProfileProjection::BrowserWasm`,
/// and the overlay's banner states plainly that native providers require
/// desktop or a companion process. This is the literal browser-side half of
/// "BrowserWasm shows native providers explicitly as requires desktop or
/// companion" (BENCH_SPEC §6).
#[wasm_bindgen_test]
async fn overlay_renders_the_runtime_honest_browser_unavailable_state() {
    let host = mount();
    settle().await;

    click(&query(&host, "[data-slot=\"feeds\"]").expect("feeds slot"));
    settle().await;

    let banner = query(&host, "[data-testid=\"extensions-banner\"]")
        .expect("the overlay renders a runtime banner");
    let text = banner.text_content().unwrap_or_default();
    assert!(
        text.contains("unavailable") && text.contains("requires desktop or a companion process"),
        "BrowserWasm must show the honest native-unavailable banner; got {text:?}"
    );
}

/// Honest absence (G7 not yet projected): no live provider catalog exists
/// anywhere in this product today, so the overlay must render "No
/// providers are registered" — never a fabricated row claiming a fake OK.
#[wasm_bindgen_test]
async fn overlay_shows_honest_absence_when_no_provider_catalog_is_projected() {
    let host = mount();
    settle().await;

    click(&query(&host, "[data-slot=\"feeds\"]").expect("feeds slot"));
    settle().await;

    assert!(
        query(&host, "[data-testid=\"extensions-empty\"]").is_some(),
        "no live catalog exists yet — the overlay must show honest absence"
    );
    assert!(
        query(&host, "[data-testid=\"extensions-list\"]").is_none(),
        "no fabricated provider rows may render"
    );
}

/// Esc closes the Extensions overlay through the shell's ordinary
/// one-at-a-time overlay ladder — no bespoke close path.
#[wasm_bindgen_test]
async fn escape_closes_the_extensions_overlay() {
    let host = mount();
    settle().await;

    click(&query(&host, "[data-slot=\"feeds\"]").expect("feeds slot"));
    settle().await;
    let shell_root = query(&host, ".dna-shell").expect("shell root mounts");
    assert!(query(&host, "[data-overlay=\"extensions\"]").is_some());

    press_chord(&shell_root.clone().unchecked_into(), "Escape");
    settle().await;

    assert!(
        query(&host, "[data-overlay=\"extensions\"]").is_none(),
        "Esc closes the Extensions overlay"
    );
}

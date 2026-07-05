//! H11: browser-test harness bootstrap (`wasm-bindgen-test --headless`).
//!
//! One proof test per shell route, imitating the "fixture -> drive -> assert"
//! shape of `dnatreecalc-host/tests/grid_interest_dispatch.rs`, adapted to the
//! browser DOM instead of a bare `leptos::prelude::Owner`:
//!
//! - `app_mounts_into_host_element`: `mount_dnatreecalc` mounts the shell into
//!   a real `<div>` and produces visible DOM content (the mount proof).
//! - `demo_grid_renders_and_cell_click_selects`: with `?grid=1` in the URL
//!   (the dev affordance `mount_dnatreecalc` reads), the demo grid attaches
//!   (`TreeWorkspaceSession::attach_demo_grid`), the Sheet lens renders its
//!   `.dtc-grid` surface with windowed cells, and clicking a node row drives
//!   `SelectNode` through the live dispatcher to a `dtc-sel--selected` class.
//!
//! These are smoke tests only (H11 NON-goals: no coverage targets, no visual
//! regression, no CI wiring, no exhaustive per-component tests) -- each test
//! proves one shell route mounts/reacts, not that every surface is correct.
//!
//! Run locally (headless is the runner's default; NO_HEADLESS=1 opts out):
//!
//! ```text
//! cargo test -p dnatreecalc-web --target wasm32-unknown-unknown
//! ```
//!
//! The repo-level `.cargo/config.toml` wires the wasm32 target's runner to
//! `wasm-bindgen-test-runner` (from the globally installed wasm-bindgen-cli,
//! version-matched to this workspace's pinned `wasm-bindgen`), per
//! docs/ux/FRONTEND_UI_DESIGN_AND_ROUTEMAP.md §F.7.
//!
//! One-time local setup: the runner needs a WebDriver/browser pair, resolved
//! via the `GECKODRIVER`/`CHROMEDRIVER` env vars (the repo `.cargo/config.toml`
//! pins `GECKODRIVER=geckodriver`, resolved on PATH) or PATH auto-detection:
//! - Firefox: drop `geckodriver.exe` (github.com/mozilla/geckodriver/releases)
//!   somewhere on PATH, e.g. `~/.cargo/bin` (H11 installed geckodriver 0.37.0
//!   against Firefox 151 this way).
//! - Chrome: same with `chromedriver`, version-matched to the installed Chrome.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Let queued reactive/rendering work flush (one macrotask turn).
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

/// Point the page URL's query string at `search` (e.g. `"?grid=1"` or `""`)
/// without reloading, preserving path + hash so the wasm-bindgen-test harness
/// page keeps working. `mount_dnatreecalc` reads `location.search` for its
/// `?grid=1` / `?worker=1` dev affordances.
fn set_url_search(search: &str) {
    let window = web_sys::window().expect("window");
    let location = window.location();
    let url = format!(
        "{}{}{}",
        location.pathname().expect("pathname"),
        search,
        location.hash().expect("hash"),
    );
    window
        .history()
        .expect("history")
        .replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url))
        .expect("replaceState");
}

/// Create a fresh `<div id="...">` under `<body>` and return it. Each test
/// gets its own element id and its own `localStorage` clear so mounts never
/// see a previous test's persisted workspace/selection.
fn fresh_mount_point(id: &str) -> web_sys::HtmlElement {
    let window = web_sys::window().expect("window");
    if let Ok(Some(storage)) = window.local_storage() {
        let _ = storage.clear();
    }
    let document = window.document().expect("document");
    let element = document
        .create_element("div")
        .expect("create mount div")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("HtmlElement");
    element.set_id(id);
    document
        .body()
        .expect("document body")
        .append_child(&element)
        .expect("append mount div");
    element
}

/// Query inside the mounted host only — tests share one page, so global
/// selectors could hit a previous test's app.
fn query_in(host: &web_sys::Element, selector: &str) -> Option<web_sys::Element> {
    host.query_selector(selector).expect("query_selector")
}

/// Mount proof: `mount_dnatreecalc` targets a real `<div>` and the shell
/// renders visible content into it. Break this by mounting to a nonexistent
/// element id (the bead's mutation check) and this test goes red.
#[wasm_bindgen_test]
async fn app_mounts_into_host_element() {
    set_url_search("");
    let host = fresh_mount_point("dtc-h11-mount-app");
    assert_eq!(host.child_element_count(), 0, "mount point starts empty");

    dnatreecalc_web::mount_dnatreecalc("dtc-h11-mount-app")
        .expect("mount_dnatreecalc must succeed");
    next_tick().await;

    assert!(
        host.child_element_count() > 0,
        "mount_dnatreecalc must render content into the host element"
    );
    assert!(
        query_in(&host, ".dtc-shell").is_some(),
        "the mounted shell must render the workspace shell"
    );
}

/// Demo-grid smoke: `?grid=1` attaches the demo grid at mount
/// (`TreeWorkspaceSession::attach_demo_grid`); switching to the Sheet lens
/// (the same lens Ctrl+4 reaches, driven here through the skin-switcher tab)
/// renders the `.dtc-grid` surface with windowed cells; and clicking an
/// unselected node row dispatches `SelectNode` through the live dispatcher,
/// which lands as the `dtc-sel--selected` class on that row.
///
/// This fails if `attach_demo_grid` breaks (no `.dtc-grid`/cells render) or
/// if the click→selection path breaks (no `dtc-sel--selected` lands) — the
/// bead's mutation checks exercise both legs.
#[wasm_bindgen_test]
async fn demo_grid_renders_and_cell_click_selects() {
    set_url_search("?grid=1");
    let host = fresh_mount_point("dtc-h11-mount-grid");

    dnatreecalc_web::mount_dnatreecalc("dtc-h11-mount-grid")
        .expect("mount_dnatreecalc must succeed");
    next_tick().await;

    // Switch the main lens to Sheet via its skin-switcher tab (registry order
    // may change; find the tab by its display name).
    let tabs = host
        .query_selector_all(".dtc-skin-switcher__tab")
        .expect("query tabs");
    let mut sheet_tab = None;
    for index in 0..tabs.length() {
        let Some(node) = tabs.item(index) else {
            continue;
        };
        let element: web_sys::HtmlElement = node.dyn_into().expect("tab is an HtmlElement");
        if element.text_content().unwrap_or_default().contains("Sheet") {
            sheet_tab = Some(element);
            break;
        }
    }
    sheet_tab
        .expect("the shell must render a Sheet lens tab")
        .click();
    next_tick().await;

    // The demo grid must render as a windowed grid surface.
    assert!(
        query_in(&host, ".dtc-grid").is_some(),
        "?grid=1 must render the demo grid surface in the Sheet lens"
    );
    assert!(
        query_in(&host, ".dtc-grid__cell").is_some(),
        "the demo grid must render at least one windowed cell"
    );

    // Click an unselected node row and prove the selection lands. (`?grid=1`
    // preselects the grid's own node, so pick a row that is NOT selected yet
    // to observe a real state change.)
    let rows = host
        .query_selector_all("[data-node-key]")
        .expect("query rows");
    assert!(rows.length() > 0, "the Sheet lens must render node rows");
    let mut unselected = None;
    for index in 0..rows.length() {
        let Some(node) = rows.item(index) else {
            continue;
        };
        let element: web_sys::HtmlElement = node.dyn_into().expect("row is an HtmlElement");
        if !element.class_name().contains("dtc-sel--selected") {
            unselected = Some(element);
            break;
        }
    }
    let row = unselected.expect("at least one node row must start unselected");

    row.click();
    next_tick().await;

    assert!(
        row.class_name().contains("dtc-sel--selected"),
        "clicking a node row must select it (dtc-sel--selected)"
    );
}

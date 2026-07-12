//! Shared scaffolding for the wasm-bindgen browser corpus.
//!
//! Every per-surface invariant module reuses these helpers so individual
//! tests stay short and the mount / teardown discipline is enforced in
//! one place. Each test is responsible for invoking
//! [`MountedShell::tear_down`] (or letting `Drop` do it) before the
//! function returns; otherwise mounted DOM nodes from earlier tests can
//! leak into the live document and confuse `query_selector`.

#![cfg(target_arch = "wasm32")]
#![allow(dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};

use dnaonecalc_host::app::host_mount::{bootstrap_editor_bridge, HostMountTarget};
use dnaonecalc_host::app::preview_state::preview_minimal_host_state;
use dnaonecalc_host::ui::components::home_shell::HomeShell;
use leptos::mount::mount_to;
use leptos::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

static HOST_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Yield to the JS microtask queue so reactive updates flush before
/// the next assertion. Browser tests interleave DOM reads with
/// awaits on this helper.
pub async fn next_microtask() {
    JsFuture::from(web_sys::js_sys::Promise::resolve(&JsValue::UNDEFINED))
        .await
        .expect("microtask tick");
}

/// Yield N microtasks in sequence. Convenient for waiting on multi-step
/// reactive cascades (e.g. input event → reducer → bridge → re-render).
pub async fn flush_microtasks(count: usize) {
    for _ in 0..count {
        next_microtask().await;
    }
}

/// Resolve the active document. Panics if the test runs without a window
/// (it never does in headless Edge, but the explicit panic is friendlier
/// than the implicit Option-unwrap).
pub fn document() -> web_sys::Document {
    web_sys::window()
        .expect("window")
        .document()
        .expect("document")
}

/// One mounted home-shell instance. Created via [`mount_home_shell`];
/// dropped (or torn down explicitly) at the end of the test.
pub struct MountedShell {
    pub host: web_sys::Element,
    pub host_id: String,
}

impl MountedShell {
    /// Detach the shell from the live document. Call before the test
    /// returns to keep subsequent tests' `query_selector(...)` lookups
    /// unambiguous.
    pub fn tear_down(self) {
        self.host.remove();
    }

    /// Locate the shell's textarea via the home-shell CSS class. Panics
    /// when not found because every home-shell render owns a textarea.
    pub async fn textarea(&self) -> web_sys::HtmlTextAreaElement {
        let document = document();
        for _ in 0..30 {
            if let Some(element) = self
                .host
                .query_selector(".onecalc-home-shell__textarea")
                .expect("query ok")
            {
                return element
                    .dyn_into::<web_sys::HtmlTextAreaElement>()
                    .expect("textarea cast");
            }
            // Document-wide fallback in case the host wrapper hasn't
            // attached yet on the first tick.
            if let Some(element) = document
                .query_selector(".onecalc-home-shell__textarea")
                .expect("query ok")
            {
                return element
                    .dyn_into::<web_sys::HtmlTextAreaElement>()
                    .expect("textarea cast");
            }
            next_microtask().await;
        }
        panic!("textarea did not mount within 30 microtask ticks");
    }

    /// Run a query selector scoped to this shell's host element.
    pub fn select(&self, selector: &str) -> Option<web_sys::Element> {
        self.host.query_selector(selector).expect("query ok")
    }

    /// Run a query selector returning a NodeList scoped to this shell.
    pub fn select_all(&self, selector: &str) -> web_sys::NodeList {
        self.host.query_selector_all(selector).expect("query ok")
    }
}

/// Mount a fresh home shell with the seeded preview state and the live
/// editor bridge. Each call attaches its own host `<div>` keyed by an
/// incrementing counter, so concurrent leftover state from prior tests
/// never collides.
pub fn mount_home_shell() -> MountedShell {
    let document = document();
    let id = HOST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let host_id = format!("onecalc-browser-test-host-{id}");
    if let Some(existing) = document.get_element_by_id(&host_id) {
        existing.remove();
    }
    let host = document.create_element("div").expect("host element");
    host.set_id(&host_id);
    document
        .body()
        .expect("body")
        .append_child(&host)
        .expect("append host");

    let initial_state = preview_minimal_host_state();
    let editor_bridge = bootstrap_editor_bridge(HostMountTarget::WebBrowser);
    let host_element: web_sys::HtmlElement = host.clone().unchecked_into();
    let mount_handle = mount_to(host_element, move || {
        view! {
            <HomeShell
                initial_state=initial_state.clone()
                editor_bridge=Some(editor_bridge.clone())
            />
        }
    });
    // Match `lib.rs::mount_onecalc_preview` — once the shell is mounted,
    // detach the unmount handle so the reactive runtime keeps the shell
    // alive for the duration of the test. `tear_down` removes the host
    // node, which is what cleans up between tests.
    std::mem::forget(mount_handle);

    MountedShell { host, host_id }
}

/// Set the textarea's value and dispatch a real `input` event so the
/// component's `on:input` handler fires through
/// `services::live_edit::apply_live_editor_input`. Selection is set to
/// the end of the new value (the most common authoring case).
pub fn dispatch_input(textarea: &web_sys::HtmlTextAreaElement, value: &str) {
    let caret = value.chars().count() as u32;
    textarea.set_value(value);
    textarea
        .set_selection_range(caret, caret)
        .expect("set selection range");
    textarea
        .dispatch_event(&web_sys::InputEvent::new("input").expect("input event"))
        .expect("dispatch input event");
}

/// Poll a query-selector-text predicate until it matches, or up to
/// `max_ticks` microtasks have elapsed. Returns the matched text on
/// success (trimmed), or the last seen text on timeout. The 30-tick
/// default is enough for one bridge round-trip in headless Edge with a
/// live `NativeOxfmlHostSession`.
pub async fn wait_for_text(shell: &MountedShell, selector: &str, expected: &str) -> Option<String> {
    for _ in 0..30 {
        next_microtask().await;
        if let Some(element) = shell.select(selector) {
            let text = element.text_content().unwrap_or_default();
            let trimmed = text.trim();
            if trimmed == expected {
                return Some(trimmed.to_string());
            }
        }
    }
    shell
        .select(selector)
        .map(|el| el.text_content().unwrap_or_default().trim().to_string())
}

/// Poll a query-selector predicate that returns `Option<T>`; resolves on
/// the first `Some` or after `max_ticks` microtasks.
pub async fn wait_for<T>(
    shell: &MountedShell,
    selector: &str,
    predicate: impl Fn(&web_sys::Element) -> Option<T>,
) -> Option<T> {
    for _ in 0..30 {
        next_microtask().await;
        if let Some(element) = shell.select(selector) {
            if let Some(value) = predicate(&element) {
                return Some(value);
            }
        }
    }
    None
}

/// Trim helper: pull the live text content of the first match for
/// `selector`, trimmed of surrounding whitespace.
pub fn text_of(shell: &MountedShell, selector: &str) -> Option<String> {
    shell
        .select(selector)
        .map(|el| el.text_content().unwrap_or_default().trim().to_string())
}

/// Dispatch a synthetic keydown event with the given `key` value
/// (matches `KeyboardEvent.key`, e.g. "ArrowDown", "Tab", "Escape").
/// `cancelable: true` so `preventDefault` calls actually take effect
/// when the application's keydown handler intercepts the event.
///
/// Synthetic key events do NOT trigger the browser's native textarea
/// behaviour (e.g. arrow keys do not move the caret) — they DO
/// trigger any JS-installed `on:keydown` handlers, which is exactly
/// what the popup keyboard policy tests need to assert.
pub fn dispatch_keydown(textarea: &web_sys::HtmlTextAreaElement, key: &str) {
    dispatch_keydown_with_modifiers(textarea, key, false, false, false);
}

/// Same as [`dispatch_keydown`] but with explicit modifier flags
/// (Ctrl / Shift / Alt). Used for the chord-driven invariants
/// like `Ctrl+D` (formula drill) and `Ctrl+Shift+M` (compare jump).
pub fn dispatch_keydown_with_modifiers(
    textarea: &web_sys::HtmlTextAreaElement,
    key: &str,
    ctrl: bool,
    shift: bool,
    alt: bool,
) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_ctrl_key(ctrl);
    init.set_shift_key(shift);
    init.set_alt_key(alt);
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
        .expect("keydown event");
    textarea.dispatch_event(&event).expect("dispatch keydown");
}

/// Dispatch a synthetic focusout event on the textarea.
pub fn dispatch_focusout(textarea: &web_sys::HtmlTextAreaElement) {
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event =
        web_sys::Event::new_with_event_init_dict("focusout", &init).expect("focusout event");
    textarea.dispatch_event(&event).expect("dispatch focusout");
}

/// Read the popup's `data-selected-index` attribute as a `usize`.
/// `None` when the popup is not mounted.
pub fn popup_selected_index(shell: &MountedShell) -> Option<usize> {
    shell
        .select(".onecalc-completion-popup")
        .and_then(|el| el.get_attribute("data-selected-index"))
        .and_then(|s| s.parse().ok())
}

/// Read the popup's `data-item-count` attribute as a `usize`. `None`
/// when the popup is not mounted; `Some(0)` is theoretically possible
/// but the auto-open policy never produces it (zero items keep the
/// state Hidden, which suppresses the popup div entirely).
pub fn popup_item_count(shell: &MountedShell) -> Option<usize> {
    shell
        .select(".onecalc-completion-popup")
        .and_then(|el| el.get_attribute("data-item-count"))
        .and_then(|s| s.parse().ok())
}

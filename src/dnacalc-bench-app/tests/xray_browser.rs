//! Browser suite for the Bench X-Ray (BENCH_SPEC §4, bead dtc-lfz.4).
//!
//! Mounts the real [`BenchApp`] over the real OxFml host and drives the X-Ray
//! at full fidelity in a live headless browser: F8 opens the dockable panel and
//! renders drill rows from a REAL OneFormula projection; selecting a
//! subexpression raises the partial-evaluation pill; `]` rings the evaluated
//! span outward; a spilling formula renders its array result windowed with a
//! shape readout. Every assertion is a behaviour the pre-fix build did not
//! have (F8 was unbound, there was no panel, no pill, no windowed grid), so the
//! suite fails against pre-fix code — verified by stashing the implementation.

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

/// Let the OxFml pass + re-projection + remount settle.
async fn settle() {
    next_tick().await;
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

fn count(host: &web_sys::HtmlElement, selector: &str) -> u32 {
    host.query_selector_all(selector).unwrap().length()
}

fn textarea(host: &web_sys::HtmlElement) -> web_sys::HtmlTextAreaElement {
    query(host, ".dna-bridge__input")
        .expect("bridge textarea")
        .unchecked_into()
}

fn set_textarea(host: &web_sys::HtmlElement, text: &str) {
    let area = textarea(host);
    area.set_value(text);
    area.dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
}

/// Set the textarea selection to `[start, end)` (UTF-16 units) and fire the
/// `select` event the editor listens on.
fn set_selection(host: &web_sys::HtmlElement, start: u32, end: u32) {
    let area = textarea(host);
    area.set_selection_start(Some(start)).unwrap();
    area.set_selection_end(Some(end)).unwrap();
    area.dispatch_event(&web_sys::Event::new("select").unwrap())
        .unwrap();
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

/// The `data-xray-node` of the single ringed drill row, if any.
fn ringed_node(host: &web_sys::HtmlElement) -> Option<String> {
    query(host, ".dna-xray__row[data-ring=\"true\"]")
        .and_then(|row| row.get_attribute("data-xray-node"))
}

/// §4 — F8 toggles the dockable X-Ray panel; it renders drill rows with state
/// chips from a real OneFormula projection, addressable by stable node id.
#[wasm_bindgen_test]
async fn f8_opens_xray_and_renders_real_drill_rows() {
    let host = mount();
    next_tick().await;
    set_textarea(&host, "=SUM(1,2,3)");
    settle().await;

    // Closed by default.
    assert!(
        query(&host, "[data-testid=\"xray-panel\"]").is_none(),
        "the X-Ray panel is closed until F8"
    );

    // F8 from inside the editor (function-key exempt) opens the panel.
    press_chord(&textarea(&host).unchecked_into(), "F8");
    settle().await;

    let panel = query(&host, "[data-testid=\"xray-panel\"]").expect("F8 opens the X-Ray panel");
    assert!(
        panel.query_selector(".dna-xray__row").unwrap().is_some(),
        "the panel renders drill rows from the real projection"
    );
    // Rows are addressable (mechanism 20) and the root is the SUM formula.
    let root = panel
        .query_selector(".dna-xray__row[data-xray-node=\"drill-node:0\"]")
        .unwrap()
        .expect("root drill row is addressable by stable node id");
    assert!(
        root.query_selector(".dna-xray__chip").unwrap().is_some(),
        "each drill row carries a state chip"
    );
    // A real evaluated node reports its Evaluated state verbatim.
    assert!(
        count(&host, ".dna-xray__chip[data-state=\"evaluated\"]") >= 1,
        "at least one node evaluated"
    );

    // F8 again closes it (ToggleFormulaDrill round-trip).
    press_chord(&textarea(&host).unchecked_into(), "F8");
    settle().await;
    assert!(
        query(&host, "[data-testid=\"xray-panel\"]").is_none(),
        "F8 toggles the panel closed"
    );
}

/// §4 — selecting a subexpression raises the partial-evaluation pill showing
/// that node's value, in place, without opening the panel or mutating the
/// formula.
#[wasm_bindgen_test]
async fn selecting_a_subexpression_raises_the_partial_eval_pill() {
    let host = mount();
    next_tick().await;
    set_textarea(&host, "=SUM(1,2,3)");
    settle().await;

    assert!(
        query(&host, ".dna-bridge__pill").is_none(),
        "no pill while the selection is collapsed"
    );

    // Select the literal `1` at bytes [5,6) (ASCII => UTF-16 units 5..6).
    set_selection(&host, 5, 6);
    settle().await;

    let pill = query(&host, ".dna-bridge__pill").expect("selection raises the pill");
    let text = pill.text_content().unwrap_or_default();
    assert!(
        text.contains('1'),
        "the pill shows the selected subexpression's value; got {text:?}"
    );
    // The pill points at a stable, addressable node (mechanism 20).
    assert!(
        pill.get_attribute("data-pill-node")
            .is_some_and(|id| id.starts_with("drill-node:")),
        "the pill carries the matched drill node id"
    );
    // The formula text is untouched — the pill is a read, not a rewrite.
    assert_eq!(textarea(&host).value(), "=SUM(1,2,3)");

    // Collapsing the selection dismisses the pill.
    set_selection(&host, 11, 11);
    settle().await;
    assert!(
        query(&host, ".dna-bridge__pill").is_none(),
        "collapsing the selection dismisses the pill"
    );
}

/// §4 — `]` rings the evaluated span outward: the panel mirrors the current
/// ring, and a widen step moves the highlight to the enclosing expression.
#[wasm_bindgen_test]
async fn ring_out_widens_the_highlighted_drill_row() {
    let host = mount();
    next_tick().await;
    set_textarea(&host, "=SUM(1,2,3)");
    settle().await;

    // Put the caret inside the innermost literal, then open the X-Ray so the
    // ring seeds at that deep node.
    set_selection(&host, 5, 5);
    settle().await;
    press_chord(&textarea(&host).unchecked_into(), "F8");
    settle().await;

    let panel = query(&host, "[data-testid=\"xray-panel\"]").expect("panel open");
    let seeded =
        ringed_node(&host).expect("the ring seeds at the caret's node when the panel opens");

    // `]` reaches the shell as TraceForward when focus is on the panel (a
    // non-text-entry) — inside the editor it would be a formula character.
    press_chord(&panel.clone().unchecked_into(), "]");
    settle().await;

    assert_eq!(
        count(&host, ".dna-xray__row[data-ring=\"true\"]"),
        1,
        "exactly one drill row mirrors the current ring"
    );
    let widened = ringed_node(&host).expect("a row is still ringed after widening");
    assert_ne!(
        widened, seeded,
        "ring-out moved the highlight to the enclosing expression"
    );
}

/// §3 — a spilling formula renders its array result windowed, with a shape
/// readout; opening the X-Ray shows the same spill as a windowed drill preview.
#[wasm_bindgen_test]
async fn array_result_renders_windowed_with_shape_readout() {
    let host = mount();
    next_tick().await;
    set_textarea(&host, "=SEQUENCE(3,3)");
    settle().await;

    let array = query(&host, "[data-result=\"array\"]").expect("array result renders");
    assert!(
        array
            .text_content()
            .unwrap_or_default()
            .contains("3\u{00D7}3"),
        "the result carries a shape readout"
    );
    // The window is a real 3×3 grid of host-projected cells.
    assert_eq!(
        count(&host, ".bench-result__grid-row"),
        3,
        "three windowed rows render"
    );
    assert_eq!(
        count(&host, ".bench-result__grid-cell"),
        9,
        "nine windowed cells render"
    );

    // The X-Ray drill mirrors the spill as a windowed preview grid.
    press_chord(&textarea(&host).unchecked_into(), "F8");
    settle().await;
    assert!(
        query(&host, "[data-testid=\"xray-panel\"] .dna-xray__grid").is_some(),
        "the drill preview renders the spill windowed too"
    );
}

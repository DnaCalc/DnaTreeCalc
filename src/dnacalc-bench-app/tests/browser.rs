//! Browser suite for the DNA Bench app (SHELL_SPEC §10.1-10.4, D3 parity).
//!
//! Mounts the real [`BenchApp`] in a live headless browser and proves, in the
//! DOM, that the Bench composition mounts its region subset, that a formula is
//! authored end-to-end over the real OxFml host (tokens + result + commit +
//! revert), that the keyboard atlas and command deck open from the registry,
//! and that the reserved parity slot renders NOTHING (the Evidence slot is
//! proven in the shell suite).

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

fn shell_root(host: &web_sys::HtmlElement) -> web_sys::EventTarget {
    host.query_selector(".dna-shell")
        .unwrap()
        .expect("shell root mounts")
        .unchecked_into()
}

fn set_textarea(host: &web_sys::HtmlElement, text: &str) {
    let area = query(host, ".dna-bridge__input").expect("bridge textarea");
    let area: &web_sys::HtmlTextAreaElement = area.unchecked_ref();
    area.set_value(text);
    area.dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
}

fn press_chord(target: &web_sys::EventTarget, key: &str, ctrl: bool, alt: bool, shift: bool) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_ctrl_key(ctrl);
    init.set_alt_key(alt);
    init.set_shift_key(shift);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    target.dispatch_event(&event).unwrap();
}

/// §10.1 — the Bench composition mounts its region subset (no Registry rail),
/// and the reserved parity slot renders nothing.
#[wasm_bindgen_test]
fn bench_mounts_region_subset_without_registry() {
    let host = mount();

    for region in ["mast", "bridge", "stage-host", "inspector", "strip"] {
        assert!(
            query(&host, &format!("[data-region=\"{region}\"]")).is_some(),
            "region {region} must mount in the Bench composition"
        );
    }
    // Bench omits the Registry rail entirely (BENCH_SPEC §2) — first-class,
    // not a hidden-but-present region.
    assert!(
        query(&host, "[data-region=\"registry\"]").is_none(),
        "Bench must not compose a Registry rail"
    );

    // The reserved parity slot renders NOTHING (PARITY_TRUST_UX §8).
    let parity = query(&host, "[data-slot=\"parity\"]").expect("parity slot reserved in layout");
    assert_eq!(parity.text_content().unwrap_or_default(), "");
    assert_eq!(parity.child_element_count(), 0);
}

/// §10.2 — author a formula end-to-end over the REAL OxFml host: the token
/// underlay lights, the Result stage shows the computed value, commit holds it,
/// and revert restores the committed text.
#[wasm_bindgen_test]
async fn bench_authors_formula_end_to_end() {
    let host = mount();
    next_tick().await;

    // The bridge mounts in FULL mode (real token underlay, not degrade).
    assert!(
        query(&host, ".dna-bridge--full").is_some(),
        "Bench mounts the FULL-fidelity bridge"
    );

    // Type a formula and let the OxFml pass + re-projection settle.
    set_textarea(&host, "=SUM(1,2,3)");
    next_tick().await;
    next_tick().await;

    // Real syntax runs: the function token is present as a colored span.
    assert!(
        query(&host, ".dna-bridge__seg--role-function").is_some(),
        "OxFml syntax runs light the function token in the underlay"
    );

    // The Result stage shows the computed value (6) from host truth.
    let value = query(&host, "[data-testid=\"bench-result\"] .bench-result__value")
        .expect("result value renders");
    assert_eq!(value.text_content().unwrap_or_default(), "6");

    // Commit (Enter on the textarea → CommitRequested); the value holds.
    let area = query(&host, ".dna-bridge__input").expect("textarea");
    press_chord(&area.unchecked_into(), "Enter", false, false, false);
    next_tick().await;
    let value = query(&host, "[data-testid=\"bench-result\"] .bench-result__value")
        .expect("result value after commit");
    assert_eq!(value.text_content().unwrap_or_default(), "6");

    // Edit away, then revert (Esc) restores the committed formula's result.
    set_textarea(&host, "=1+1");
    next_tick().await;
    next_tick().await;
    let area = query(&host, ".dna-bridge__input").expect("textarea");
    press_chord(&area.unchecked_into(), "Escape", false, false, false);
    next_tick().await;
    next_tick().await;
    let value = query(&host, "[data-testid=\"bench-result\"] .bench-result__value")
        .expect("result value after revert");
    assert_eq!(
        value.text_content().unwrap_or_default(),
        "6",
        "EscapeRevert restores the committed =SUM(1,2,3) result"
    );
}

/// §10.3 — the keyboard atlas opens from the live registry.
#[wasm_bindgen_test]
async fn bench_atlas_opens_from_registry() {
    let host = mount();
    next_tick().await;
    press_chord(&shell_root(&host), "/", true, false, false);
    next_tick().await;
    let atlas = query(&host, "[data-overlay=\"keyboard-atlas\"]").expect("atlas overlay opens");
    assert!(
        atlas.query_selector("[data-atlas-row]").unwrap().is_some(),
        "the atlas lists chords from the live registry"
    );
}

/// §10.4 — the command deck opens and lists at least 15 commands.
#[wasm_bindgen_test]
async fn bench_command_deck_lists_at_least_fifteen_commands() {
    let host = mount();
    next_tick().await;
    press_chord(&shell_root(&host), "k", true, false, false);
    next_tick().await;
    let deck = query(&host, "[data-overlay=\"command-deck\"]").expect("command deck opens");
    let rows = deck.query_selector_all("[data-command-id]").unwrap();
    assert!(
        rows.length() >= 15,
        "the deck surfaces >= 15 commands (SHELL_SPEC §10.4), got {}",
        rows.length()
    );
}

/// Beads dtc-lfz.3 + dtc-lfz.9 — the command deck's Save/Open rows track the
/// REAL capability the app advertises, not a hardcoded value. On the browser
/// (wasm32) target the split is now honest and asymmetric:
///   - Save still renders disabled-with-reason (no browser download adapter is
///     wired at this layer yet — the host projects `can_save = false`).
///   - Open renders ENABLED: the app now owns a real `<input type=file>`
///     workspace picker (bead dtc-lfz.9), so `shell_persistence_from_host`
///     advertises `can_open = true` on wasm. This is the DOM-visible proof that
///     the enablement flip is driven by the app's real picker seam, not a stub
///     (before dtc-lfz.9 `can_open` was force-honest-false and this row was
///     disabled).
#[wasm_bindgen_test]
async fn bench_command_deck_save_disabled_but_open_enabled_on_browser() {
    let host = mount();
    next_tick().await;
    press_chord(&shell_root(&host), "k", true, false, false);
    next_tick().await;
    let deck = query(&host, "[data-overlay=\"command-deck\"]").expect("command deck opens");

    let save = deck
        .query_selector("[data-command-id=\"shell.save\"]")
        .unwrap()
        .expect("save row present");
    assert_eq!(
        save.get_attribute("data-enabled").as_deref(),
        Some("false"),
        "browser has no real Save adapter wired yet — must render honestly disabled"
    );
    assert_eq!(save.get_attribute("aria-disabled").as_deref(), Some("true"));
    assert!(
        save.get_attribute("title")
            .unwrap_or_default()
            .contains("does not yet support"),
        "the disabled reason must be honest, not blank"
    );

    let open = deck
        .query_selector("[data-command-id=\"shell.open\"]")
        .unwrap()
        .expect("open row present");
    assert_eq!(
        open.get_attribute("data-enabled").as_deref(),
        Some("true"),
        "the browser now has a real <input type=file> workspace picker \
         (bead dtc-lfz.9), so Open must render enabled"
    );
    assert_ne!(
        open.get_attribute("aria-disabled").as_deref(),
        Some("true"),
        "an enabled Open row must not be aria-disabled"
    );
}

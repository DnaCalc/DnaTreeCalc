//! View-mode toggle invariants.
//!
//! The home shell carries a workspace-level reading-audience
//! preference. Default is User mode (Excel-user-friendly chrome
//! with phase chips, state slugs, and SEAM markers hidden);
//! Ctrl+Alt+D toggles to Developer mode (full engineering
//! surface).
//!
//! This corpus pins the toggle plumbing only. The mode-conditional
//! rendering of the foot chips and the walk-tree is pinned by
//! the corpora attached to the next two beads
//! (foot_chip_modes.rs and walk_tree_modes.rs).

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, dispatch_keydown_with_modifiers, mount_home_shell};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn default_view_mode_is_user_on_first_mount() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let root = shell
        .select(".onecalc-home-shell")
        .expect("home-shell root mounted");
    assert_eq!(
        root.get_attribute("data-view-mode").as_deref(),
        Some("user"),
        "fresh mount must default to User view-mode",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn ctrl_alt_d_toggles_data_view_mode_attribute() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    // Ctrl+Alt+D — User -> Developer.
    dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;
    let root = shell.select(".onecalc-home-shell").expect("root mounted");
    assert_eq!(
        root.get_attribute("data-view-mode").as_deref(),
        Some("developer"),
        "Ctrl+Alt+D must flip data-view-mode to developer",
    );

    // Ctrl+Alt+D again — Developer -> User.
    dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;
    let root = shell.select(".onecalc-home-shell").expect("root mounted");
    assert_eq!(
        root.get_attribute("data-view-mode").as_deref(),
        Some("user"),
        "second Ctrl+Alt+D must flip data-view-mode back to user",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn status_foot_dev_button_is_always_present_with_mode_attribute() {
    // Pin the button is ALWAYS rendered (so users can discover
    // Developer mode without the chord) and that data-view-mode
    // reflects the active mode. aria-pressed mirrors the same
    // signal for assistive tech.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    let button = shell
        .select(".onecalc-home-shell__statusfoot-mode-button")
        .expect("dev button rendered in User mode");
    assert_eq!(
        button.get_attribute("data-view-mode").as_deref(),
        Some("user"),
    );
    assert_eq!(
        button.get_attribute("aria-pressed").as_deref(),
        Some("false")
    );
    assert_eq!(button.text_content().unwrap_or_default().trim(), "dev");

    dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;

    let button = shell
        .select(".onecalc-home-shell__statusfoot-mode-button")
        .expect("dev button still rendered after toggle");
    assert_eq!(
        button.get_attribute("data-view-mode").as_deref(),
        Some("developer"),
    );
    assert_eq!(
        button.get_attribute("aria-pressed").as_deref(),
        Some("true")
    );

    dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;
    let button = shell
        .select(".onecalc-home-shell__statusfoot-mode-button")
        .expect("dev button after second toggle");
    assert_eq!(
        button.get_attribute("data-view-mode").as_deref(),
        Some("user"),
    );
}

#[wasm_bindgen_test(async)]
async fn ctrl_shift_d_also_toggles_view_mode() {
    // Ctrl+Shift+D is the second accepted chord for the
    // view-mode toggle. Pin that it works in addition to
    // Ctrl+Alt+D (some platforms swallow Ctrl+Alt+D for
    // accessibility shortcuts; the user confirmed in their
    // setup that Ctrl+Shift+D reaches the page).
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    dispatch_keydown_with_modifiers(&textarea, "d", true, true, false);
    super::scaffold::flush_microtasks(15).await;
    let root = shell.select(".onecalc-home-shell").expect("root");
    assert_eq!(
        root.get_attribute("data-view-mode").as_deref(),
        Some("developer"),
        "Ctrl+Shift+D must toggle view-mode (regression of Ctrl+Shift+D \
         accidentally firing the formula-drill chord)",
    );

    dispatch_keydown_with_modifiers(&textarea, "d", true, true, false);
    super::scaffold::flush_microtasks(15).await;
    let root = shell.select(".onecalc-home-shell").expect("root");
    assert_eq!(
        root.get_attribute("data-view-mode").as_deref(),
        Some("user"),
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn ctrl_shift_d_does_not_open_formula_drill_panel() {
    // Specific regression pin for the bug the user reported:
    // Ctrl+Shift+D was acting like Ctrl+D because the Ctrl+D
    // condition didn't exclude the shift modifier. The drill
    // panel must NOT open on Ctrl+Shift+D.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    dispatch_keydown_with_modifiers(&textarea, "d", true, true, false);
    super::scaffold::flush_microtasks(15).await;

    let panel = shell
        .select(".onecalc-home-shell__formula-drill-panel")
        .expect("panel rendered");
    assert_eq!(
        panel.get_attribute("data-expanded").as_deref(),
        Some("false"),
        "Ctrl+Shift+D must NOT open the formula drill (it toggles view-mode)",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn clicking_status_foot_dev_button_toggles_view_mode() {
    use wasm_bindgen::JsCast;
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let button = shell
        .select(".onecalc-home-shell__statusfoot-mode-button")
        .expect("dev button rendered");

    // Use mousedown (matches the component's on:mousedown handler).
    let init = web_sys::MouseEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    let mousedown = web_sys::MouseEvent::new_with_mouse_event_init_dict("mousedown", &init)
        .expect("mousedown event");
    button
        .dispatch_event(&mousedown)
        .expect("dispatch mousedown");
    super::scaffold::flush_microtasks(15).await;

    let root = shell.select(".onecalc-home-shell").expect("root");
    assert_eq!(
        root.get_attribute("data-view-mode").as_deref(),
        Some("developer"),
        "clicking the dev button must toggle into Developer mode",
    );

    // Clicking again toggles back.
    let button = shell
        .select(".onecalc-home-shell__statusfoot-mode-button")
        .expect("dev button still rendered");
    let mousedown = web_sys::MouseEvent::new_with_mouse_event_init_dict("mousedown", &init)
        .expect("mousedown event");
    button
        .dispatch_event(&mousedown)
        .expect("dispatch mousedown");
    super::scaffold::flush_microtasks(15).await;

    let root = shell.select(".onecalc-home-shell").expect("root");
    assert_eq!(
        root.get_attribute("data-view-mode").as_deref(),
        Some("user"),
    );

    let _ = button.dyn_ref::<web_sys::HtmlElement>(); // silence unused-warning
    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn ctrl_d_alone_still_toggles_drill_not_view_mode() {
    // Pin that Ctrl+D (no Alt) keeps its existing meaning
    // (formula drill toggle) and does NOT flip the view-mode.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    dispatch_keydown_with_modifiers(&textarea, "d", true, false, false);
    super::scaffold::flush_microtasks(15).await;

    // Drill panel is now expanded.
    let panel = shell
        .select(".onecalc-home-shell__formula-drill-panel")
        .expect("panel rendered");
    assert_eq!(
        panel.get_attribute("data-expanded").as_deref(),
        Some("true"),
    );

    // View-mode is still User (Ctrl+D alone did not flip it).
    let root = shell.select(".onecalc-home-shell").expect("root mounted");
    assert_eq!(
        root.get_attribute("data-view-mode").as_deref(),
        Some("user"),
        "Ctrl+D (no Alt) must NOT flip the view-mode",
    );

    shell.tear_down();
}

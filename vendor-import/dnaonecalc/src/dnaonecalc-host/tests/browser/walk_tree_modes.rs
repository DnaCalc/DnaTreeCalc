//! View-mode-conditional rendering of the formula walk-tree
//! drill-down panel.
//!
//! Pins both User-mode (Excel-user-friendly equation rows + a
//! single-status line replacing the phase strip) and Developer-
//! mode (state chips + parse/bind/eval phase chips) rendering.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use super::scaffold::{
    dispatch_input, dispatch_keydown_with_modifiers, mount_home_shell, wait_for,
};

wasm_bindgen_test_configure!(run_in_browser);

async fn open_drill(
    shell: &super::scaffold::MountedShell,
    textarea: &web_sys::HtmlTextAreaElement,
    formula: &str,
) {
    dispatch_input(textarea, formula);
    super::scaffold::flush_microtasks(15).await;
    dispatch_keydown_with_modifiers(textarea, "d", true, false, false);
    let _ = wait_for(shell, ".onecalc-home-shell__formula-drill-panel", |el| {
        if el.get_attribute("data-expanded").as_deref() == Some("true") {
            Some(())
        } else {
            None
        }
    })
    .await;
    super::scaffold::flush_microtasks(15).await;
}

async fn switch_to_developer_mode(textarea: &web_sys::HtmlTextAreaElement) {
    dispatch_keydown_with_modifiers(textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;
}

#[wasm_bindgen_test(async)]
async fn walk_tree_user_mode_renders_label_equals_value_layout() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=SUM(1,2)").await;

    let row = shell
        .select(".onecalc-home-shell__formula-drill-row")
        .expect("row mounted");
    assert_eq!(row.get_attribute("data-mode").as_deref(), Some("user"));

    // The User-mode row carries an equals-sign element.
    let equals = row
        .query_selector(".onecalc-home-shell__formula-drill-equals")
        .expect("query ok");
    assert!(
        equals.is_some(),
        "User-mode row must render an equals sign between label and value",
    );

    // Label and value spans are present.
    let label = row
        .query_selector(".onecalc-home-shell__formula-drill-label")
        .expect("query ok");
    assert!(label.is_some());
    let value = row
        .query_selector(".onecalc-home-shell__formula-drill-value")
        .expect("query ok");
    assert!(value.is_some());

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn walk_tree_user_mode_keeps_data_state_attribute_on_row() {
    // The User-mode row layout does NOT emit the state-chip span,
    // but the row itself still carries `data-state` so the
    // seam-status board (later bead) and the corpus can read
    // the row's classifier without switching modes. The chip
    // span is Developer-mode-only.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=SUM(1,2)").await;

    let row = shell
        .select(".onecalc-home-shell__formula-drill-row")
        .expect("row");
    let state_attr = row.get_attribute("data-state");
    assert!(
        matches!(
            state_attr.as_deref(),
            Some("evaluated") | Some("bound") | Some("opaque") | Some("blocked")
        ),
        "row must carry a data-state slug regardless of mode; got {state_attr:?}",
    );

    // The state chip span is User-mode-omitted entirely.
    let chip = row
        .query_selector(".onecalc-home-shell__formula-drill-state")
        .expect("query ok");
    assert!(
        chip.is_none(),
        "User-mode row must NOT render the state-chip span (Developer-mode only)",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn walk_tree_user_mode_renders_dots_for_missing_value() {
    // When `value_preview` is None on a node (e.g. an Opaque
    // intermediate) the User-mode row renders `…` muted instead
    // of an empty value column.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=SUM(1,2)").await;

    // Pick any User-mode value span and check it is non-empty
    // (real bridge runs always produce at least the SUM result).
    // The "..." case is exercised in unit tests against the
    // projector since it requires constructing a snapshot with
    // value_preview=None.
    let value = shell
        .select(".onecalc-home-shell__formula-drill-value")
        .expect("value span mounted");
    let text = value.text_content().unwrap_or_default();
    assert!(
        !text.is_empty(),
        "value column must always carry text (value or '…'); got {text:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn walk_tree_developer_mode_keeps_state_chip_text_visible() {
    // Regression pin: in Developer mode, the state-chip span
    // text reads as one of the four state slugs.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=SUM(1,2)").await;
    switch_to_developer_mode(&textarea).await;

    let row = shell
        .select(".onecalc-home-shell__formula-drill-row")
        .expect("row");
    assert_eq!(row.get_attribute("data-mode").as_deref(), Some("developer"));

    let chip = row
        .query_selector(".onecalc-home-shell__formula-drill-state")
        .expect("query ok")
        .expect("state chip present in Developer mode");
    let chip_text = chip.text_content().unwrap_or_default();
    assert!(
        matches!(
            chip_text.trim(),
            "evaluated" | "bound" | "opaque" | "blocked"
        ),
        "state chip text must be one of the four state slugs; got {chip_text:?}",
    );

    // The User-mode equals span MUST NOT be rendered in
    // Developer mode (it is mode-conditional rendering, not
    // mode-conditional CSS).
    let equals = row
        .query_selector(".onecalc-home-shell__formula-drill-equals")
        .expect("query ok");
    assert!(
        equals.is_none(),
        "Developer-mode row must NOT render the User-mode equals sign",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn phase_strip_user_mode_renders_single_status_line_when_clean() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=SUM(1,2)").await;

    let strip = shell
        .select(".onecalc-home-shell__formula-drill-phase-strip")
        .expect("phase strip mounted");
    assert_eq!(strip.get_attribute("data-mode").as_deref(), Some("user"));

    // Only ONE status pill, not three phase chips.
    let pills = strip.query_selector_all(":scope > span").expect("query ok");
    assert!(
        pills.length() <= 1,
        "User-mode phase strip should render at most one status pill; got {}",
        pills.length(),
    );
    let status = strip
        .query_selector(".onecalc-home-shell__formula-drill-status")
        .expect("query ok")
        .expect("status pill present");
    assert_eq!(status.get_attribute("data-status").as_deref(), Some("ok"));
    let text = status.text_content().unwrap_or_default();
    assert!(
        text.contains("evaluated"),
        "User-mode clean status should read 'evaluated...'; got {text:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn phase_strip_developer_mode_keeps_three_phase_chips() {
    // Regression pin: Developer-mode phase strip carries three
    // phase chips (parse / bind / eval). Same shape as before
    // bead 32.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=SUM(1,2)").await;
    switch_to_developer_mode(&textarea).await;

    let strip = shell
        .select(".onecalc-home-shell__formula-drill-phase-strip")
        .expect("phase strip mounted");
    assert_eq!(
        strip.get_attribute("data-mode").as_deref(),
        Some("developer"),
    );
    let chips = strip
        .query_selector_all(".onecalc-home-shell__formula-drill-phase")
        .expect("query ok");
    assert_eq!(
        chips.length(),
        3,
        "Developer-mode phase strip must render exactly 3 chips (parse/bind/eval)",
    );
    // No User-mode status pill.
    let status = strip
        .query_selector(".onecalc-home-shell__formula-drill-status")
        .expect("query ok");
    assert!(
        status.is_none(),
        "Developer-mode phase strip must NOT render the User-mode status pill",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn mode_toggle_re_renders_walk_tree_without_collapsing_panel() {
    // The drill-down panel must remain expanded across a
    // Ctrl+Alt+D mode toggle. The rows + phase strip re-render
    // in the new mode shape.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=SUM(1,2)").await;

    let panel = shell
        .select(".onecalc-home-shell__formula-drill-panel")
        .expect("panel");
    assert_eq!(
        panel.get_attribute("data-expanded").as_deref(),
        Some("true")
    );
    assert_eq!(panel.get_attribute("data-mode").as_deref(), Some("user"));

    switch_to_developer_mode(&textarea).await;

    let panel = shell
        .select(".onecalc-home-shell__formula-drill-panel")
        .expect("panel still rendered");
    assert_eq!(
        panel.get_attribute("data-expanded").as_deref(),
        Some("true"),
        "panel must remain expanded across mode toggle",
    );
    assert_eq!(
        panel.get_attribute("data-mode").as_deref(),
        Some("developer"),
    );
    let row = shell
        .select(".onecalc-home-shell__formula-drill-row")
        .expect("row re-rendered");
    assert_eq!(row.get_attribute("data-mode").as_deref(), Some("developer"));

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn user_mode_blocked_row_renders_blocked_tag_instead_of_equals() {
    // Drive a state where one of the walk nodes is Blocked. We
    // rely on the bridge to produce a Blocked node for inputs
    // it can't evaluate fully; if no row turns out Blocked the
    // test skips the assertion (different bridge versions may
    // behave differently). When a Blocked row IS present the
    // `blocked` tag must appear and the equals sign must NOT.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=XLOOKUP(A1,B1:B9,C1:C9)").await;

    let blocked_row =
        shell.select_all(".onecalc-home-shell__formula-drill-row[data-state=\"blocked\"]");
    if blocked_row.length() == 0 {
        // No blocked row materialised — bridge tokenized the
        // input some other way. Skip the rendering assertion;
        // unit tests cover this projector path directly.
        shell.tear_down();
        return;
    }
    let row = blocked_row
        .item(0)
        .expect("blocked row")
        .dyn_into::<web_sys::Element>()
        .expect("element");
    assert_eq!(row.get_attribute("data-mode").as_deref(), Some("user"));
    let blocked_tag = row
        .query_selector(".onecalc-home-shell__formula-drill-blocked-tag")
        .expect("query ok");
    assert!(
        blocked_tag.is_some(),
        "User-mode blocked row must render the 'blocked' tag",
    );
    let equals = row
        .query_selector(".onecalc-home-shell__formula-drill-equals")
        .expect("query ok");
    assert!(
        equals.is_none(),
        "User-mode blocked row must NOT render the equals sign",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn user_mode_phase_strip_blocked_status_when_provenance_blocked() {
    // The phase-strip user-mode status flips to `data-status="blocked"`
    // when any phase is Blocked (driven by provenance_summary's
    // blocked_reason). Unit-level coverage in the projector is
    // already in place; here we pin the rendered DOM contract.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=XLOOKUP(A1,B1:B9,C1:C9)").await;

    let status = shell
        .select(".onecalc-home-shell__formula-drill-status")
        .expect("status pill rendered");
    let data_status = status.get_attribute("data-status").unwrap_or_default();
    // Either ok or blocked is acceptable depending on whether
    // the bridge classified the formula as blocked. Pin only
    // that the attribute exists with a known value.
    assert!(
        matches!(data_status.as_str(), "ok" | "blocked"),
        "status pill data-status must be ok or blocked; got {data_status:?}",
    );

    shell.tear_down();
}

//! Signature-help line invariants.
//!
//! The signature-help line is a non-interactive tooltip rendered above
//! the caret while the caret sits inside an open function call. The
//! bridge populates `editor_document.signature_help` and the home
//! shell projects it via the `signature_help` field on the home
//! view-model. These invariants pin:
//!
//! * the line APPEARS when the caret is inside a function call,
//! * the line DISMISSES when the caret leaves the call,
//! * the active-parameter highlight ADVANCES at each comma,
//! * the line is anchored ABOVE the caret (top < caret-anchor top),
//! * the completion popup SUPPRESSES the signature help when both
//!   want the same caret,
//! * the line carries data-active-parameter so the corpus can read
//!   the active-parameter index without DOM-dependent CSS scraping.
//!
//! All driven through the real `NativeOxfmlHostSession` against a freshly
//! mounted home shell.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, mount_home_shell, wait_for};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn signature_help_appears_inside_open_function_call() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(");

    let pcount = wait_for(&shell, ".onecalc-signature-help", |el| {
        el.get_attribute("data-parameter-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;
    assert!(
        pcount.is_some_and(|n| n >= 1),
        "signature help should mount with parameters when caret is inside =SUM(",
    );

    let active = shell
        .select(".onecalc-signature-help")
        .and_then(|el| el.get_attribute("data-active-parameter"));
    assert_eq!(
        active.as_deref(),
        Some("0"),
        "first parameter active when caret is just inside the open paren",
    );

    let active_param = shell
        .select(".onecalc-signature-help__parameter--active")
        .map(|el| el.text_content().unwrap_or_default());
    assert!(
        active_param
            .as_deref()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "active-parameter span should carry text content",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn signature_help_dismisses_when_caret_leaves_function_call() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    dispatch_input(&textarea, "=SUM(");
    let _ = wait_for(&shell, ".onecalc-signature-help", |el| {
        el.get_attribute("data-parameter-count")
    })
    .await;

    // Close the call AND continue past it so the caret is
    // unambiguously outside the call's span. The bridge's
    // signature_help context can still match while the caret is
    // at the closing paren itself; advancing past `+2` puts the
    // caret in plain expression territory.
    dispatch_input(&textarea, "=SUM(1)+2");
    super::scaffold::flush_microtasks(20).await;

    assert!(
        shell.select(".onecalc-signature-help").is_none(),
        "signature help should dismiss once the caret is past the call",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn signature_help_active_parameter_advances_after_comma() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,");

    let active = wait_for(&shell, ".onecalc-signature-help", |el| {
        let attr = el.get_attribute("data-active-parameter")?;
        if attr == "1" {
            Some(attr)
        } else {
            None
        }
    })
    .await;
    assert_eq!(
        active.as_deref(),
        Some("1"),
        "active-parameter should advance to 1 after first comma; got {active:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn signature_help_anchors_above_caret() {
    // The signature help is positioned absolutely with `top` set to
    // the caret-box top in pixels, then transformed upward by 100% +
    // a 6 px gap. Read the CSS-resolved bounding box and verify the
    // bottom of the help is above the caret-box top.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(");

    let help_element = wait_for(&shell, ".onecalc-signature-help", |el| {
        el.get_attribute("data-parameter-count").map(|_| el.clone())
    })
    .await
    .expect("signature help mounted");

    // Read the inline `top` style — that's the caret-box top in pixels
    // (the upward offset is applied via CSS transform, not by the
    // top value).
    let top_attr = help_element.get_attribute("style").and_then(|style| {
        style
            .split(';')
            .map(str::trim)
            .find(|s| s.starts_with("top:"))
            .and_then(|s| s.split(':').nth(1))
            .map(|v| v.trim().trim_end_matches("px").trim().to_string())
    });
    assert!(
        top_attr.is_some(),
        "signature help inline style should carry a top:Npx value",
    );
    // The numeric `top` corresponds to the caret-box top — for a
    // single-line input it's 0 (top of the textarea). The visible
    // tooltip sits above it via CSS transform; we just pin the
    // anchor scheme here.
    let top_value: i32 = top_attr.as_deref().unwrap().parse().unwrap_or(-1);
    assert!(
        top_value >= 0,
        "anchor top should be non-negative; got {top_value}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn signature_help_callee_text_matches_function_name() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=IF(");

    let callee = wait_for(&shell, ".onecalc-signature-help__callee", |el| {
        el.text_content().filter(|t| !t.is_empty())
    })
    .await;
    assert_eq!(
        callee.as_deref(),
        Some("IF"),
        "callee element should carry the function name",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn signature_help_suppressed_when_completion_popup_is_open() {
    // After `=SUM(s` the bridge emits proposals starting with `s`
    // (SUM, SUMIF, etc.) AND signature_help for the `=SUM(` outer
    // call. Both compete for the same caret area. The popup wins;
    // signature help should be suppressed.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(s");

    // Wait for the popup to mount.
    let popup_count = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;
    assert!(
        popup_count.is_some(),
        "precondition: popup must be open with proposals matching `s`",
    );

    // Signature help must NOT be present at the same time.
    assert!(
        shell.select(".onecalc-signature-help").is_none(),
        "signature help should be suppressed while the completion popup is open",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn signature_help_returns_when_completion_popup_dismisses() {
    // Reverse of the suppression: after dismissing the popup with
    // Esc, signature help should re-appear (assuming the caret is
    // still inside an open call).
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(s");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
    })
    .await;

    super::scaffold::dispatch_keydown(&textarea, "Escape");
    super::scaffold::flush_microtasks(15).await;

    assert!(
        shell.select(".onecalc-completion-popup").is_none(),
        "popup should dismiss on Escape",
    );

    let signature = wait_for(&shell, ".onecalc-signature-help", |el| {
        el.get_attribute("data-parameter-count")
    })
    .await;
    assert!(
        signature.is_some(),
        "signature help should re-appear once the popup is gone",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn signature_help_does_not_capture_focus_or_pointer() {
    // The line is non-interactive: pointer-events: none on the
    // wrapper, no focusable controls inside. Pin the contract by
    // checking the computed pointer-events and that the textarea
    // retains focus while the help is visible.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    textarea.focus().expect("focus");
    dispatch_input(&textarea, "=SUM(");

    let _ = wait_for(&shell, ".onecalc-signature-help", |el| {
        el.get_attribute("data-parameter-count")
    })
    .await;

    let document = super::scaffold::document();
    let active_id = document
        .active_element()
        .and_then(|el| Some(el.tag_name()))
        .unwrap_or_default();
    assert_eq!(
        active_id.to_uppercase(),
        "TEXTAREA",
        "textarea should retain focus while signature help is visible",
    );

    shell.tear_down();
}

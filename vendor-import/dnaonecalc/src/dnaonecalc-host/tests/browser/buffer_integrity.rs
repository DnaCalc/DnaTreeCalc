//! Buffer-integrity matrix.
//!
//! For a wide range of formula-text inputs, dispatch the input
//! through the live bridge and assert the syntax-overlay text
//! equals the textarea value character-for-character. This is
//! the rendering-side contract the WS-14 home shell depends on.
//!
//! Each entry is its own `#[wasm_bindgen_test]` so a failure
//! localises to a specific input. Inputs known to fail
//! upstream (today: any input that the OxFml editor tokenizer
//! drops or fails to tile) are marked `#[ignore = "..."]` with
//! a reason naming the upstream surface and pointing at the
//! relevant `docs/handoffs/...` file. Once an upstream fix
//! lands, removing the `#[ignore]` is the regression gate.
//!
//! Why parameterise this way: the user-reported `\n=aaa` and
//! `= a a` bugs both surfaced because nobody had typed those
//! exact strings in a test. The matrix here makes each input
//! a first-class citizen of the corpus so tokenizer gaps
//! surface immediately rather than waiting for the next
//! lucky keystroke. Adding a new input is one line.
//!
//! Convention: each test is named `case_<slug>` where the slug
//! describes the input shape briefly. The body delegates to a
//! single `assert_overlay_matches_textarea_for(text)` helper
//! so the per-test code is a single line.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, mount_home_shell};

wasm_bindgen_test_configure!(run_in_browser);

/// Mount, dispatch the input, and assert the syntax overlay's
/// text content equals the textarea value character-for-
/// character (modulo the explicit trailing newline the overlay
/// emits to keep its line-box height).
async fn assert_overlay_matches_textarea_for(input: &str) {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, input);
    super::scaffold::flush_microtasks(20).await;

    let textarea_value = textarea.value();
    assert_eq!(
        textarea_value, input,
        "precondition: textarea must contain the dispatched input verbatim",
    );

    let overlay_text_raw = shell
        .select(".onecalc-home-shell__editor-overlay")
        .and_then(|el| el.text_content())
        .unwrap_or_default();
    let overlay_stripped = overlay_text_raw
        .strip_suffix('\n')
        .map(|s| s.to_string())
        .unwrap_or(overlay_text_raw.clone());

    assert_eq!(
        overlay_stripped,
        textarea_value,
        "buffer-integrity: overlay text must match textarea value \
         character-for-character. \
         input = {input:?}, overlay = {overlay_stripped:?} \
         ({} chars), textarea = {textarea_value:?} ({} chars).",
        overlay_stripped.chars().count(),
        textarea_value.chars().count(),
    );

    shell.tear_down();
}

// =============================================================
// Cases that pass today
// =============================================================

#[wasm_bindgen_test(async)]
async fn case_eq_alone() {
    assert_overlay_matches_textarea_for("=").await;
}

#[wasm_bindgen_test(async)]
async fn case_eq_sum_no_args() {
    assert_overlay_matches_textarea_for("=SUM()").await;
}

#[wasm_bindgen_test(async)]
async fn case_eq_sum_two_args() {
    assert_overlay_matches_textarea_for("=SUM(1,2)").await;
}

#[wasm_bindgen_test(async)]
async fn case_eq_sum_with_inner_space() {
    assert_overlay_matches_textarea_for("= SUM(1,2)").await;
}

#[wasm_bindgen_test(async)]
async fn case_eq_a1() {
    assert_overlay_matches_textarea_for("=A1").await;
}

#[wasm_bindgen_test(async)]
async fn case_eq_a1_plus_b1() {
    assert_overlay_matches_textarea_for("=A1+B1").await;
}

#[wasm_bindgen_test(async)]
async fn case_eq_nested_calls() {
    assert_overlay_matches_textarea_for("=SUM(IF(1,2,3),4)").await;
}

#[wasm_bindgen_test(async)]
async fn case_eq_with_text_literal() {
    assert_overlay_matches_textarea_for("=\"hello\"").await;
}

#[wasm_bindgen_test(async)]
async fn case_literal_number_no_eq() {
    assert_overlay_matches_textarea_for("42").await;
}

#[wasm_bindgen_test(async)]
async fn case_literal_text_no_eq() {
    assert_overlay_matches_textarea_for("hello").await;
}

#[wasm_bindgen_test(async)]
async fn case_forced_text_apostrophe() {
    assert_overlay_matches_textarea_for("'42").await;
}

#[wasm_bindgen_test(async)]
async fn case_eq_then_open_paren_only() {
    // Diagnostic: this input is a known parse error but the
    // textarea contents must still render in full.
    assert_overlay_matches_textarea_for("=SUM(").await;
}

// =============================================================
// Cases that fail today (pending upstream OxFml fix)
//
// Each carries a reason string that names the upstream surface
// and points at the relevant docs/handoffs/... file. Once the
// upstream fix lands, removing the #[ignore] is the regression
// gate.
// =============================================================

#[wasm_bindgen_test(async)]
async fn case_eq_space_a_space_a() {
    assert_overlay_matches_textarea_for("= a a").await;
}

#[wasm_bindgen_test(async)]
async fn case_eq_space_a_space_b_space_c() {
    assert_overlay_matches_textarea_for("= a b c").await;
}

#[wasm_bindgen_test(async)]
async fn case_a1_space_b1_no_eq() {
    // Cell-ref + space + identifier with no leading `=`. Same
    // class — the inter-token whitespace is dropped.
    assert_overlay_matches_textarea_for("A1 B1").await;
}

#[wasm_bindgen_test(async)]
async fn case_eq_multi_space_between_identifiers() {
    assert_overlay_matches_textarea_for("=ABC   DEF").await;
}

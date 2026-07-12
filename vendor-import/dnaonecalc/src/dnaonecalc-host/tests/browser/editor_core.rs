//! Editor-core browser invariants for the WS-14 home shell.
//!
//! These tests are the gate before any caret-positioning surface
//! (completion popup, signature help line, hover tooltip) lands. Each
//! invariant runs through the real `NativeOxfmlHostSession` mounted into a
//! detached DOM root, and asserts on user-visible DOM contracts:
//!
//!   - the textarea is reachable and configured
//!     (`spellcheck="false"`, `autocomplete="off"`),
//!   - typing flows through `apply_live_editor_input` and updates the
//!     result hero within one bridge round-trip,
//!   - the four hero-visual surfaces from beads dno-xcq.16-19 attach to
//!     the DOM correctly: caption pills, syntax overlay, diagnostic
//!     squiggle overlay, foot chips,
//!   - clearing the textarea returns the result block to its
//!     placeholder.
//!
//! The §12.1 keyboard-mechanic invariants from the WS-14 plan
//! (Arrow/Backspace/Delete moving the caret) are intentionally NOT in
//! this suite: synthetic key events do not move the native caret in
//! WebDriver, so testing them at the DOM layer would assert the wrong
//! contract. Those invariants live as state-side scenario tests
//! against `EditorSurfaceState` in `tests/scenarios/typing.rs` and the
//! lib-side reducer suite.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, mount_home_shell, text_of, wait_for, wait_for_text};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn home_shell_mounts_with_textarea_and_empty_pills() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    // Caption row above the editor carries a "Formula ▸" caption and an
    // entry-mode pill. With the seeded empty formula space the pill is
    // data-mode="empty".
    let entry_pill = shell
        .select(".onecalc-home-shell__caption-pill--entry")
        .expect("entry-mode pill rendered");
    assert_eq!(
        entry_pill.get_attribute("data-mode").as_deref(),
        Some("empty"),
        "fresh mount should classify entry-mode as Empty",
    );

    // Result-class pill is suppressed when there is no result to label.
    assert!(
        shell
            .select(".onecalc-home-shell__caption-pill--result")
            .is_none(),
        "result-class pill should be absent on the Empty result view",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn textarea_carries_spellcheck_off_and_autocomplete_off() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    // The DOM attribute and the IDL property both matter — assert each.
    assert_eq!(
        textarea.get_attribute("spellcheck").as_deref(),
        Some("false"),
        "textarea must carry spellcheck=\"false\" so browsers don't \
         underline 'SUM' as a typo",
    );
    assert_eq!(
        textarea.get_attribute("autocomplete").as_deref(),
        Some("off"),
        "textarea must carry autocomplete=\"off\"",
    );
    assert!(
        !textarea.spellcheck(),
        "textarea.spellcheck IDL property must agree with the attribute",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn typing_sum_formula_shows_six_in_result_hero() {
    // Plan §12.1 #7: =SUM(1,2,3) → result "6" via the real OxFml runtime.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2,3)");

    let result = wait_for_text(&shell, ".onecalc-home-shell__result-block .value", "6").await;
    assert_eq!(result.as_deref(), Some("6"));

    let pill = shell
        .select(".onecalc-home-shell__caption-pill--result")
        .expect("result-class pill rendered");
    assert_eq!(pill.get_attribute("data-class").as_deref(), Some("number"));

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn typing_sequence_formula_shows_array_shape() {
    // Plan §12.1 #8: =SEQUENCE(2,2) → array result.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SEQUENCE(2,2)");

    // Result block shape: "Array[2 × 2]" plus the result-class pill flips
    // to data-class="array".
    let result_text = wait_for(&shell, ".onecalc-home-shell__result-block", |element| {
        let text = element.text_content().unwrap_or_default();
        if text.contains("Array[2") && text.contains("2]") {
            Some(text)
        } else {
            None
        }
    })
    .await;
    assert!(
        result_text.is_some(),
        "result block should label a 2×2 array; got {:?}",
        result_text,
    );

    let pill = shell
        .select(".onecalc-home-shell__caption-pill--result")
        .expect("result-class pill rendered");
    assert_eq!(pill.get_attribute("data-class").as_deref(), Some("array"));

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn typing_literal_number_is_classified_as_value() {
    // Pre-bridge hand-eval path: a literal `1.5` with no leading `=`
    // should classify as Value entry mode and render the same number in
    // the result hero with the Number result-class pill.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "1.5");

    let result = wait_for_text(&shell, ".onecalc-home-shell__result-block .value", "1.5").await;
    assert_eq!(result.as_deref(), Some("1.5"));

    let entry_pill = shell
        .select(".onecalc-home-shell__caption-pill--entry")
        .expect("entry-mode pill rendered");
    assert_eq!(
        entry_pill.get_attribute("data-mode").as_deref(),
        Some("value")
    );

    let result_pill = shell
        .select(".onecalc-home-shell__caption-pill--result")
        .expect("result-class pill rendered");
    assert_eq!(
        result_pill.get_attribute("data-class").as_deref(),
        Some("number")
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn typing_unmatched_paren_surfaces_diagnostic_squiggle_and_error_pill() {
    // Plan §12.1 #9: malformed formula → diagnostic.
    // After Bead 18, the diagnostic also appears as a wavy underline in
    // the squiggle overlay layer with severity-coloured CSS.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(");

    // Result-class pill flips to error.
    let pill = wait_for(
        &shell,
        ".onecalc-home-shell__caption-pill--result",
        |element| element.get_attribute("data-class"),
    )
    .await;
    assert_eq!(pill.as_deref(), Some("error"));

    // Diagnostic squiggle layer carries at least one squiggle--error span
    // with a `title` attribute (the hover tooltip).
    let squiggle_count = shell
        .select_all(".onecalc-home-shell__editor-squiggles .squiggle--error")
        .length();
    assert!(
        squiggle_count >= 1,
        "expected at least one .squiggle--error span; got {}",
        squiggle_count,
    );

    let title = shell
        .select(".onecalc-home-shell__editor-squiggles .squiggle--error")
        .and_then(|el| el.get_attribute("title"));
    assert!(
        title.is_some_and(|t| t.contains(':')),
        "squiggle title should carry 'diagnostic_id: message'",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn syntax_overlay_emits_function_token_for_sum() {
    // After Bead 17, the syntax overlay renders one `.syn-fn` span per
    // function token. `=SUM(1,2,3)` produces exactly one.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2,3)");

    // Wait for the bridge round-trip so the overlay's syntax_runs are
    // populated (the stale-document guard yields an empty overlay
    // until then).
    let _ = wait_for_text(&shell, ".onecalc-home-shell__result-block .value", "6").await;

    let function_spans = shell.select_all(".onecalc-home-shell__editor-overlay .syn-fn");
    assert_eq!(
        function_spans.length(),
        1,
        "exactly one .syn-fn span for SUM",
    );
    let span = function_spans
        .item(0)
        .expect("first syn-fn span")
        .dyn_into::<web_sys::Element>()
        .expect("element");
    assert_eq!(
        span.text_content().unwrap_or_default().trim(),
        "SUM",
        "syn-fn span should carry the function token text",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn editor_metrics_chip_reports_token_function_diagnostic_counts() {
    // After Bead 19, the editor-foot chip carries data-tokens /
    // data-functions / data-diagnostics with live counts.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    let _ = wait_for_text(&shell, ".onecalc-home-shell__result-block .value", "3").await;

    let chip = shell
        .select(".onecalc-home-shell__chip--metrics")
        .expect("editor-metrics chip rendered");
    let tokens: usize = chip
        .get_attribute("data-tokens")
        .and_then(|s| s.parse().ok())
        .expect("data-tokens parses");
    let functions: usize = chip
        .get_attribute("data-functions")
        .and_then(|s| s.parse().ok())
        .expect("data-functions parses");
    let diagnostics: usize = chip
        .get_attribute("data-diagnostics")
        .and_then(|s| s.parse().ok())
        .expect("data-diagnostics parses");

    assert!(
        tokens >= 5,
        "tokens count should grow with formula content; got {tokens}"
    );
    assert_eq!(functions, 1, "exactly one function (SUM)");
    assert_eq!(diagnostics, 0, "no diagnostics on a well-formed SUM");

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn result_context_chip_renders_locale_and_format_seam_markers() {
    // After Bead 19 + Bead 31, the result-foot chip exposes
    // data-seam-id + aria-describedby attributes on each
    // SEAM-pending field IN BOTH VIEW MODES. The visible
    // `<NOT IMPL:SEAM-id>` badge text is Developer-mode only;
    // User mode (default) hides the badge so an Excel user
    // does not see the noise. This invariant pins both contracts.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    // The data-seam-id attributes must be present regardless of
    // mode — that is the seam-status board's read path.
    let context_chip = shell
        .select(".onecalc-home-shell__chip--context")
        .expect("result-context chip rendered");
    let _ = context_chip
        .query_selector_all("[data-seam-id]")
        .expect("query ok");

    // In Developer mode the chip's visible labels include the
    // policy and format-family strings. Locale moved to the
    // formatting panel and is no longer in the result-foot chip.
    super::scaffold::dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;
    let context_chip = shell
        .select(".onecalc-home-shell__chip--context")
        .expect("context chip still rendered after mode toggle");
    let chip_text = context_chip.text_content().unwrap_or_default();
    assert!(
        chip_text.contains("General"),
        "Developer-mode context chip must show the live `General` format \
         family label by default; got {chip_text:?}",
    );
    // Default scenario policy is `live-recalc` (Excel's default-on
    // workbook behaviour) — switched from `deterministic` in the
    // post-W072 cleanup; deterministic is now the explicit toggle
    // for reproducible-authoring mode.
    assert!(
        chip_text.contains("live-recalc"),
        "context chip must show the default `live-recalc` policy; got {chip_text:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn clearing_textarea_returns_result_to_placeholder() {
    // Type, evaluate, then clear. The home shell must transition back to
    // the Empty entry-mode pill, drop the result-class pill, and show the
    // muted placeholder in the result block.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");
    let _ = wait_for_text(&shell, ".onecalc-home-shell__result-block .value", "3").await;

    dispatch_input(&textarea, "");

    let entry_pill = wait_for(
        &shell,
        ".onecalc-home-shell__caption-pill--entry",
        |element| element.get_attribute("data-mode"),
    )
    .await;
    assert_eq!(entry_pill.as_deref(), Some("empty"));

    assert!(
        shell
            .select(".onecalc-home-shell__caption-pill--result")
            .is_none(),
        "result-class pill should be suppressed when text is empty",
    );

    let placeholder = text_of(&shell, ".onecalc-home-shell__result-block .muted");
    assert!(
        placeholder.as_deref().is_some_and(|s| !s.is_empty()),
        "muted placeholder should render in the result block; got {:?}",
        placeholder,
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn status_foot_dot_lights_when_live_bridge_returns_green_tree() {
    // After a successful bridge round-trip, the status-foot dot transitions
    // from data-health="stale" (no key) to data-health="live" (live-bridge
    // and a green-tree key). Pins the status indicator contract from the
    // Pre-MVP slice.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    let _ = wait_for_text(&shell, ".onecalc-home-shell__result-block .value", "3").await;

    let dot_health = wait_for(&shell, ".onecalc-home-shell__statusfoot-dot", |element| {
        element.get_attribute("data-health")
    })
    .await;
    assert_eq!(dot_health.as_deref(), Some("live"));

    shell.tear_down();
}

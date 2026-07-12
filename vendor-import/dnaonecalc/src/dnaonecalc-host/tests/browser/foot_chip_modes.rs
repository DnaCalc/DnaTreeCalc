//! View-mode-conditional rendering of the foot chips.
//!
//! Pins both User-mode (Excel-user-friendly) and Developer-mode
//! (full counts + SEAM badges) rendering of the editor-foot
//! live-metrics chip and the result-foot active-context chip.
//! User mode is the default; Ctrl+Alt+D flips to Developer mode.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, dispatch_keydown_with_modifiers, mount_home_shell};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn editor_metrics_chip_user_mode_omits_when_textarea_empty() {
    // No document, no input — the chip is omitted entirely so
    // the editor-foot does not carry meaningless content. The
    // formula-drill toggle button is still rendered alongside
    // the empty chip slot.
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;
    super::scaffold::flush_microtasks(15).await;

    assert!(
        shell.select(".onecalc-home-shell__chip--metrics").is_none(),
        "User mode + empty textarea must omit the metrics chip",
    );
}

#[wasm_bindgen_test(async)]
async fn editor_metrics_chip_user_mode_shows_ready_when_no_diagnostics() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");
    super::scaffold::flush_microtasks(15).await;

    let chip = shell
        .select(".onecalc-home-shell__chip--metrics")
        .expect("metrics chip rendered");
    assert_eq!(chip.get_attribute("data-mode").as_deref(), Some("user"));
    assert_eq!(chip.get_attribute("data-status").as_deref(), Some("ready"),);
    assert_eq!(chip.text_content().unwrap_or_default().trim(), "ready",);
    assert!(chip
        .class_list()
        .contains("onecalc-home-shell__chip--ready"));
}

#[wasm_bindgen_test(async)]
async fn editor_metrics_chip_user_mode_warns_when_diagnostic_present() {
    // `=SUM(` produces a diagnostic; the user-mode chip should
    // surface it as "1 issue: <message>".
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(");
    super::scaffold::flush_microtasks(15).await;

    let chip = shell
        .select(".onecalc-home-shell__chip--metrics")
        .expect("metrics chip rendered");
    assert_eq!(chip.get_attribute("data-mode").as_deref(), Some("user"));
    assert_eq!(
        chip.get_attribute("data-status").as_deref(),
        Some("diagnostic"),
    );
    assert!(chip
        .class_list()
        .contains("onecalc-home-shell__chip--warning"));
    let text = chip.text_content().unwrap_or_default();
    assert!(
        text.contains("issue"),
        "warning chip text should contain the word 'issue'; got {text:?}",
    );
    // The chip should NOT contain the developer-mode literal "tokens" / "functions".
    assert!(
        !text.contains("tokens "),
        "user mode must NOT show developer-mode counts; got {text:?}",
    );
}

#[wasm_bindgen_test(async)]
async fn editor_metrics_chip_data_attributes_present_in_both_modes() {
    // The data-tokens / data-functions / data-diagnostics
    // attributes carry the source-of-truth counts. They MUST be
    // readable in both view modes — the seam-status board (later
    // bead) and the test corpus rely on them.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");
    super::scaffold::flush_microtasks(15).await;

    let chip = shell
        .select(".onecalc-home-shell__chip--metrics")
        .expect("user-mode chip rendered");
    let user_tokens = chip.get_attribute("data-tokens");
    assert!(user_tokens.is_some());

    dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;

    let chip = shell
        .select(".onecalc-home-shell__chip--metrics")
        .expect("developer-mode chip rendered");
    assert_eq!(
        chip.get_attribute("data-tokens"),
        user_tokens,
        "data-tokens must be the same in both modes (source of truth)",
    );
    assert!(chip.get_attribute("data-functions").is_some());
    assert!(chip.get_attribute("data-diagnostics").is_some());
}

#[wasm_bindgen_test(async)]
async fn editor_metrics_chip_developer_mode_shows_full_counts() {
    // Regression pin: the developer-mode rendering must keep its
    // pre-bead-31 shape — `tokens N · functions M · diagnostics K`.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");
    dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;

    let chip = shell
        .select(".onecalc-home-shell__chip--metrics")
        .expect("metrics chip rendered");
    assert_eq!(
        chip.get_attribute("data-mode").as_deref(),
        Some("developer")
    );
    let text = chip.text_content().unwrap_or_default();
    assert!(text.contains("tokens"));
    assert!(text.contains("functions"));
    assert!(text.contains("diagnostics"));
}

#[wasm_bindgen_test(async)]
async fn result_context_chip_user_mode_omits_seam_badges() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;
    super::scaffold::flush_microtasks(15).await;

    let chip = shell
        .select(".onecalc-home-shell__chip--context")
        .expect("context chip rendered");
    assert_eq!(chip.get_attribute("data-mode").as_deref(), Some("user"));
    let chip_text = chip.text_content().unwrap_or_default();
    assert!(
        !chip_text.contains("NOT IMPL"),
        "User-mode context chip must NOT surface the SEAM badge text; \
         got {chip_text:?}",
    );
    assert!(
        !chip_text.contains("SEAM-"),
        "User-mode context chip must NOT surface SEAM ids in the visible \
         text; got {chip_text:?}",
    );

    // Data-seam-id attributes must still be readable for the
    // seam-status board.
    let seam_fields = chip.query_selector_all("[data-seam-id]").expect("query ok");
    assert!(
        seam_fields.length() >= 2,
        "data-seam-id attributes must be present even in User mode; got {}",
        seam_fields.length(),
    );
}

#[wasm_bindgen_test(async)]
async fn result_context_chip_developer_mode_keeps_seam_badges() {
    // Regression pin for Developer-mode rendering.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;

    let chip = shell
        .select(".onecalc-home-shell__chip--context")
        .expect("context chip rendered");
    assert_eq!(
        chip.get_attribute("data-mode").as_deref(),
        Some("developer"),
    );
    let chip_text = chip.text_content().unwrap_or_default();
    // Locale moved to the formatting-panel inner section; the
    // result-foot chip now shows just `format-family · policy`.
    // Format reads `General` until the user picks a code; policy
    // defaults to `live-recalc` (Excel's default-on workbook
    // behaviour).
    assert!(
        chip_text.contains("General"),
        "Developer-mode context chip must surface the live `General` \
         format-family label when no format code is set; got {chip_text:?}",
    );
    assert!(
        chip_text.contains("live-recalc"),
        "Developer-mode context chip must surface the default `live-recalc` \
         policy; got {chip_text:?}",
    );
}

#[wasm_bindgen_test(async)]
async fn mode_toggle_re_renders_chips_without_remount() {
    // Toggling Ctrl+Alt+D must update both chips immediately —
    // no delay, no remount artifact.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");
    super::scaffold::flush_microtasks(15).await;

    let metrics = shell
        .select(".onecalc-home-shell__chip--metrics")
        .expect("metrics chip");
    let context = shell
        .select(".onecalc-home-shell__chip--context")
        .expect("context chip");
    assert_eq!(metrics.get_attribute("data-mode").as_deref(), Some("user"));
    assert_eq!(context.get_attribute("data-mode").as_deref(), Some("user"));

    dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;

    let metrics = shell
        .select(".onecalc-home-shell__chip--metrics")
        .expect("metrics chip after toggle");
    let context = shell
        .select(".onecalc-home-shell__chip--context")
        .expect("context chip after toggle");
    assert_eq!(
        metrics.get_attribute("data-mode").as_deref(),
        Some("developer"),
    );
    assert_eq!(
        context.get_attribute("data-mode").as_deref(),
        Some("developer"),
    );
}

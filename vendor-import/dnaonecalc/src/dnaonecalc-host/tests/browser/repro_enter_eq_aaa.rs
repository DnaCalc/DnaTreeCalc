//! Regression pin for the previously-reported bug:
//!
//!   "Press <enter>, then '=' then 'aaa'. The a's don't show on
//!    the screen."
//!
//! Root cause was an upstream OxFml tokenizer truncation: when
//! the formula source text started with a leading whitespace
//! character (e.g. `\n`), the editor tokenizer emitted a snapshot
//! that ended after the FIRST non-trivia token and silently
//! dropped the remaining text. The same input WITHOUT the leading
//! newline tokenized correctly.
//!
//! Status: FIXED upstream by OxFml commit 162f224 ("Fix editor
//! token snapshot truncation"). DnaOneCalc consumes oxfml_core
//! via a path dependency, so this repo picks up the fix
//! automatically on the next cargo build. The two invariants
//! below now run as regression pins — they verify that the
//! upstream parser's snapshot covers the whole source text for
//! the leading-newline class of input.
//!
//! IMPORTANT for future agents: if these tests start failing
//! after an OxFml update, the fix belongs UPSTREAM. Do NOT add
//! host-side gap-fill logic that papers over a tokenizer
//! truncation. That route was tried and reverted before the
//! upstream fix landed; see `docs/OPERATIONS.md` §9 (Root-cause
//! Discipline) for why.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, mount_home_shell};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn typing_enter_then_eq_then_aaa_renders_all_chars_in_overlay() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    dispatch_input(&textarea, "\n");
    super::scaffold::flush_microtasks(15).await;
    assert_overlay_matches_textarea(&shell, &textarea, "after Enter");

    dispatch_input(&textarea, "\n=");
    super::scaffold::flush_microtasks(15).await;
    assert_overlay_matches_textarea(&shell, &textarea, "after Enter +=");

    dispatch_input(&textarea, "\n=aaa");
    super::scaffold::flush_microtasks(15).await;
    assert_overlay_matches_textarea(&shell, &textarea, "after Enter += + aaa");

    assert_eq!(textarea.value(), "\n=aaa");
    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn typing_eq_aaa_without_leading_newline_still_renders_correctly() {
    // Control: the same `=aaa` content WITHOUT the leading
    // newline tokenizes correctly. Kept enabled so a regression
    // in the no-leading-whitespace path is caught.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=aaa");
    super::scaffold::flush_microtasks(15).await;
    assert_overlay_matches_textarea(&shell, &textarea, "after =aaa");
    shell.tear_down();
}

fn assert_overlay_matches_textarea(
    shell: &super::scaffold::MountedShell,
    textarea: &web_sys::HtmlTextAreaElement,
    label: &str,
) {
    let textarea_value = textarea.value();
    let overlay_text_raw = shell
        .select(".onecalc-home-shell__editor-overlay")
        .and_then(|el| el.text_content())
        .unwrap_or_default();
    // The overlay appends one trailing newline to keep its line-box
    // height when the last line is empty; strip exactly that one.
    // We do NOT trim — the bug we are reproducing involves the
    // *first* character being a newline, which trim would silently
    // swallow.
    let overlay_stripped = overlay_text_raw
        .strip_suffix('\n')
        .map(|s| s.to_string())
        .unwrap_or(overlay_text_raw.clone());

    assert_eq!(
        overlay_stripped,
        textarea_value,
        "[{label}] overlay text must match textarea value character-for-character. \
         textarea = {textarea_value:?} ({} chars), \
         overlay  = {overlay_stripped:?} ({} chars).",
        textarea_value.chars().count(),
        overlay_stripped.chars().count(),
    );
}

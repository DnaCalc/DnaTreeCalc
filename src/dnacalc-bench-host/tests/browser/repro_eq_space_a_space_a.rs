//! Regression pin for the user-reported bug:
//!
//!   "Type '=<SPACE>a<SPACE>a' and check the buffer."
//!
//! Status: PENDING upstream fix in OxFml. See
//! `docs/handoffs/oxfml_inter_identifier_whitespace_gap.md`.
//!
//! Root cause: the upstream `EditorSyntaxSnapshot` for `= a a`
//! contains tokens at offsets 0, 1, 2, 4 — but the space at
//! offset 3 (between the two `a` identifiers) is unaccounted for
//! in any token's `trailing_trivia` or the next token's
//! `leading_trivia`. The host correctly concatenates the
//! snapshot's text and produces `= aa` (4 chars), one short of
//! the textarea's actual `= a a` (5 chars). The visible caret
//! drifts past the rendered text.
//!
//! Same class as the leading-whitespace truncation fixed in
//! OxFml commit 162f224. Both are violations of the implicit
//! contract that snapshot tokens + trivia tile the source text
//! character-for-character.
//!
//! IMPORTANT: do NOT re-enable this test by adding host-side
//! gap-fill logic. See `docs/OPERATIONS.md` §9 (Root-cause
//! Discipline) for why.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, mount_home_shell};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn typing_eq_space_a_space_a_renders_all_chars_in_overlay() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    dispatch_input(&textarea, "= a a");
    super::scaffold::flush_microtasks(20).await;

    let textarea_value = textarea.value();
    assert_eq!(textarea_value, "= a a");

    let overlay_text_raw = shell
        .select(".onecalc-home-shell__editor-overlay")
        .and_then(|el| el.text_content())
        .unwrap_or_default();
    let overlay_stripped = overlay_text_raw
        .strip_suffix('\n')
        .map(|s| s.to_string())
        .unwrap_or(overlay_text_raw.clone());

    if overlay_stripped == textarea_value {
        // No bug — the test passes without diagnostic noise.
        shell.tear_down();
        return;
    }

    // Mismatch — dump the spans for diagnosis.
    let spans = shell.select_all(".onecalc-home-shell__editor-overlay span");
    let mut dump: Vec<String> = Vec::new();
    for i in 0..spans.length() {
        if let Some(node) = spans.item(i) {
            if let Ok(el) = node.dyn_into::<web_sys::Element>() {
                let role = el.get_attribute("data-token-role").unwrap_or_default();
                let start = el.get_attribute("data-token-start").unwrap_or_default();
                let text = el.text_content().unwrap_or_default();
                dump.push(format!("  [role={role:?} start={start:?} text={text:?}]"));
            }
        }
    }
    panic!(
        "syntax overlay text does not match textarea value.\n  \
         textarea value = {textarea_value:?} ({} chars)\n  \
         overlay text   = {overlay_stripped:?} ({} chars)\n  \
         spans rendered ({} total):\n{}",
        textarea_value.chars().count(),
        overlay_stripped.chars().count(),
        spans.length(),
        dump.join("\n"),
    );
}

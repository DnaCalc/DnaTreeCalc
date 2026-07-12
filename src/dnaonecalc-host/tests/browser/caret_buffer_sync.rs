//! Caret + buffer never go out of sync under reactive re-renders.
//!
//! The bug these tests pin: under the WS-14 home shell's reactive
//! `prop:value` binding, every state change re-applies the textarea
//! value. Some browsers reset the caret to the end of the field on
//! ANY `node.value = …` assignment (even when the new value equals
//! the current value). The user clicks at offset 5 → bridge runs →
//! state re-renders → textarea value re-applied → caret jumps to
//! end. The user starts typing and the next character lands in the
//! wrong place.
//!
//! The fix this corpus pins: a NodeRef-driven `Effect` reads the
//! host's text + selection on every state change, compares to the
//! DOM, and writes back only when divergent. After every reactive
//! flush the textarea's `value` matches host `raw_entered_cell_text`
//! AND `selectionStart` / `selectionEnd` match the host's
//! `editor_surface_state.selection.anchor / focus`.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;
use web_sys::HtmlTextAreaElement;

use super::scaffold::{dispatch_input, mount_home_shell, next_microtask};

wasm_bindgen_test_configure!(run_in_browser);

/// Type a formula, programmatically position the caret in the
/// middle, dispatch a synthetic `click` so the home shell's caret-
/// sync handler runs the bridge, and assert the caret stayed where
/// the user put it.
#[wasm_bindgen_test]
async fn caret_position_survives_caret_sync_round_trip() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    dispatch_input(&textarea, "=SUM(1,2,3,4,5)");
    // Allow the bridge round-trip to flush.
    for _ in 0..10 {
        next_microtask().await;
    }

    // Move the caret into the middle (offset 6, between `1,` and `2`).
    textarea.set_selection_range(6, 6).expect("seed selection");
    dispatch_click(&textarea);
    for _ in 0..10 {
        next_microtask().await;
    }

    // After the bridge round-trip the textarea should still hold the
    // formula text AND the caret should be at offset 6, NOT reset to
    // the end of the field (which is the failure mode the NodeRef +
    // Effect is defending against).
    assert_eq!(textarea.value(), "=SUM(1,2,3,4,5)");
    assert_eq!(
        textarea.selection_start().expect("dom"),
        Some(6),
        "selectionStart drifted to end after caret-sync round-trip",
    );
    assert_eq!(
        textarea.selection_end().expect("dom"),
        Some(6),
        "selectionEnd drifted to end after caret-sync round-trip",
    );

    shell.tear_down();
}

/// Programmatically position the caret, then directly fire `click`
/// without changing the value. The textarea content is unchanged but
/// the bridge re-runs (caret-sync). The Effect must keep the caret
/// at the user's chosen position rather than letting any reactive
/// `prop:value` write reset it to end.
#[wasm_bindgen_test]
async fn click_at_offset_does_not_displace_caret() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    dispatch_input(&textarea, "=A1+B2+C3");
    for _ in 0..10 {
        next_microtask().await;
    }

    // User clicks at offset 3 (between `=A` and `1`).
    textarea.set_selection_range(3, 3).expect("seed selection");
    dispatch_click(&textarea);
    for _ in 0..10 {
        next_microtask().await;
    }

    assert_eq!(
        textarea.selection_start().expect("dom"),
        Some(3),
        "click at offset 3 must keep caret at 3 after caret-sync flush",
    );

    shell.tear_down();
}

/// Selection range (anchor != focus) is preserved across a caret-
/// sync round-trip. This catches the case where the Effect
/// collapses a multi-character selection by setting both anchor and
/// focus to the host's caret offset.
#[wasm_bindgen_test]
async fn selection_range_survives_caret_sync() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    dispatch_input(&textarea, "=SUM(1,2,3)");
    for _ in 0..10 {
        next_microtask().await;
    }

    // Select the `1,2,3` argument list (offsets 5..10).
    textarea.set_selection_range(5, 10).expect("seed selection");
    dispatch_click(&textarea);
    for _ in 0..10 {
        next_microtask().await;
    }

    let start = textarea.selection_start().expect("dom");
    let end = textarea.selection_end().expect("dom");
    // The caret-sync handler captures both anchor and focus from the
    // textarea on the synthesised event, so the host's selection
    // should mirror what the user picked. Both endpoints must
    // survive.
    assert_eq!(
        (start, end),
        (Some(5), Some(10)),
        "selection range collapsed across caret-sync round-trip",
    );

    shell.tear_down();
}

/// Synthesise a `click` event on the textarea so the home shell's
/// `on:click` handler fires, just like a real mouse click would.
fn dispatch_click(textarea: &HtmlTextAreaElement) {
    let init = web_sys::MouseEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event =
        web_sys::MouseEvent::new_with_mouse_event_init_dict("click", &init).expect("click event");
    textarea.dispatch_event(&event).expect("dispatch click");
}

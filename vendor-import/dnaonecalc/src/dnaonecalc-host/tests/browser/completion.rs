//! Browser invariants for the completion popup (bead dno-xcq.24).
//!
//! State-side coverage of the popup lifecycle lives in
//! `tests/scenarios/completion.rs`. This file pins DOM-level contracts:
//! the popup mounts when the bridge returns proposals, anchors at the
//! caret, surfaces `data-selected` / `data-proposal-id` attributes the
//! upcoming keyboard layer (bead .25) needs, and accepts on click.
//!
//! Keyboard-driven navigation tests will join this file as part of
//! bead .25.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use super::scaffold::{
    dispatch_focusout, dispatch_input, dispatch_keydown, mount_home_shell, popup_item_count,
    popup_selected_index, wait_for, wait_for_text,
};

wasm_bindgen_test_configure!(run_in_browser);

/// Typing `=SU` triggers the bridge to return SUM-family proposals;
/// the popup attaches to the DOM with at least one `.onecalc-completion-popup__item`
/// row and `data-item-count >= 1`.
#[wasm_bindgen_test(async)]
async fn typing_partial_function_opens_popup_in_dom() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let item_count = wait_for(&shell, ".onecalc-completion-popup", |element| {
        element
            .get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;
    assert!(
        item_count.is_some_and(|n| n >= 1),
        "popup should mount with at least one item; got {item_count:?}",
    );

    shell.tear_down();
}

/// First item carries `data-selected="true"`; the rest carry
/// `data-selected="false"`. Pins the contract bead .25's keyboard
/// navigation will toggle.
#[wasm_bindgen_test(async)]
async fn first_popup_item_is_selected_by_default() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    let items = shell.select_all(".onecalc-completion-popup__item");
    assert!(items.length() >= 1, "popup should have at least one row");

    let first = items
        .item(0)
        .expect("first item")
        .dyn_into::<web_sys::Element>()
        .expect("element");
    assert_eq!(
        first.get_attribute("data-selected").as_deref(),
        Some("true"),
        "first item starts selected",
    );

    if items.length() >= 2 {
        let second = items
            .item(1)
            .expect("second item")
            .dyn_into::<web_sys::Element>()
            .expect("element");
        assert_eq!(
            second.get_attribute("data-selected").as_deref(),
            Some("false"),
            "non-first items start unselected",
        );
    }

    shell.tear_down();
}

/// Each row exposes `data-proposal-id` + `data-kind` + glyph + label
/// so the keyboard layer (bead .25) and the future seam-status board
/// (bead .19+) can enumerate them without scraping inner HTML.
#[wasm_bindgen_test(async)]
async fn popup_items_carry_proposal_id_kind_and_glyph() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    let items = shell.select_all(".onecalc-completion-popup__item");
    assert!(items.length() >= 1);
    let first = items
        .item(0)
        .expect("first item")
        .dyn_into::<web_sys::Element>()
        .expect("element");
    assert!(
        first
            .get_attribute("data-proposal-id")
            .is_some_and(|s| !s.is_empty()),
        "data-proposal-id present and non-empty",
    );
    assert_eq!(
        first.get_attribute("data-kind").as_deref(),
        Some("function"),
        "SUM family items are functions",
    );
    let glyph = first
        .query_selector(".onecalc-completion-popup__glyph")
        .ok()
        .flatten()
        .and_then(|el| el.text_content());
    assert!(
        glyph.as_deref().is_some_and(|s| !s.trim().is_empty()),
        "kind glyph rendered",
    );
}

/// Popup is anchored within the editor frame's bounding box. The
/// browser test suite trusts the `style="left: ...; top: ..."`
/// attribute to position it; here we just assert the popup lives
/// inside `.onecalc-home-shell__editor-frame`.
#[wasm_bindgen_test(async)]
async fn popup_is_a_descendant_of_editor_frame_for_anchoring() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    let frame = shell
        .select(".onecalc-home-shell__editor-frame")
        .expect("editor frame mounted");
    let popup_inside_frame = frame
        .query_selector(".onecalc-completion-popup")
        .ok()
        .flatten();
    assert!(
        popup_inside_frame.is_some(),
        "popup must be a descendant of the editor frame so absolute positioning anchors correctly",
    );

    shell.tear_down();
}

/// Clicking a popup item splices its `insert_text` into the textarea
/// and dismisses the popup. Pins the click-to-accept path that the
/// bead's "no keyboard yet" scope makes the only acceptance route.
#[wasm_bindgen_test(async)]
async fn clicking_popup_item_replaces_text_and_closes_popup() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    // Click the first item via mousedown (the popup uses mousedown to
    // accept so the textarea retains focus).
    let first = shell
        .select(".onecalc-completion-popup__item")
        .expect("first item present");
    let mousedown_event_init = web_sys::MouseEventInit::new();
    mousedown_event_init.set_bubbles(true);
    mousedown_event_init.set_cancelable(true);
    let mousedown_event =
        web_sys::MouseEvent::new_with_mouse_event_init_dict("mousedown", &mousedown_event_init)
            .expect("create mousedown event");
    first
        .dispatch_event(&mousedown_event)
        .expect("dispatch mousedown");

    // After acceptance the textarea value should contain the
    // proposal's insert_text. For SUM family the first proposal
    // typically inserts "SUM(", so the textarea now reads "=SUM("
    // (or another SUM-prefixed form).
    let textarea_for_value = textarea.clone();
    let value_after = wait_for(&shell, ".onecalc-home-shell__textarea", move |_| {
        // After acceptance the partial 'SU' has been replaced by a
        // full function name. Wait until the textarea reads
        // '=<NAME>' for any NAME of length >= 3 (SUM is 3 chars).
        // The upstream proposal inserts only the function name (the
        // trailing `(` is a UX-side choice we'll add in a future
        // bead alongside argument-aware completion).
        let value = textarea_for_value.value();
        if value.starts_with('=') && value.len() > 3 {
            Some(value)
        } else {
            None
        }
    })
    .await
    .expect("textarea value updated after acceptance");
    assert!(
        value_after.starts_with('='),
        "acceptance should preserve the leading `=`; got {value_after:?}",
    );
    // The '=SU' partial was 3 chars; any accepted proposal makes it
    // longer (real function names start with at least SU + N).
    assert!(
        value_after.chars().count() > 3,
        "acceptance should expand the partial; got {value_after:?}",
    );

    // After bead .25 landed the suppression-after-accept rule, the
    // popup is Hidden right after click acceptance and STAYS hidden
    // through the synthetic input event the click handler dispatches.
    // Pin this contract.
    let popup_after = wait_for(&shell, ".onecalc-home-shell__editor-frame", |_| {
        if shell.select(".onecalc-completion-popup").is_none() {
            Some(())
        } else {
            None
        }
    })
    .await;
    assert!(
        popup_after.is_some(),
        "popup should be hidden after click acceptance (suppression-after-accept rule)",
    );

    shell.tear_down();
}

// ---------------------------------------------------------------------
// Keyboard policy (bead dno-xcq.25)
// ---------------------------------------------------------------------

/// ArrowDown advances `selected_index`; the popup re-renders with
/// the new selection.
#[wasm_bindgen_test(async)]
async fn arrowdown_advances_popup_selected_index() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    // Wait for popup to mount, with at least 2 items so ArrowDown
    // has somewhere to go.
    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 2)
    })
    .await;

    let initial = popup_selected_index(&shell).expect("initial selected index");
    assert_eq!(initial, 0);

    dispatch_keydown(&textarea, "ArrowDown");

    let advanced = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-selected-index")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n == 1)
    })
    .await;
    assert!(
        advanced.is_some(),
        "ArrowDown should move selected_index from 0 to 1",
    );

    shell.tear_down();
}

/// ArrowUp from index 0 wraps to the last index. Pins the wrap-around
/// behaviour the state machine implements.
#[wasm_bindgen_test(async)]
async fn arrowup_at_first_wraps_to_last_index() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let item_count = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 2)
    })
    .await
    .expect("at least 2 popup items");

    dispatch_keydown(&textarea, "ArrowUp");

    let wrapped = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-selected-index")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n == item_count - 1)
    })
    .await;
    assert!(
        wrapped.is_some(),
        "ArrowUp at index 0 should wrap to item_count - 1",
    );

    shell.tear_down();
}

/// Tab accepts the selected proposal and dismisses the popup. The
/// suppression-after-accept rule keeps the popup hidden through the
/// synthetic input event the acceptance dispatches.
#[wasm_bindgen_test(async)]
async fn tab_accepts_selected_proposal_and_closes_popup() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    dispatch_keydown(&textarea, "Tab");

    let textarea_for_value = textarea.clone();
    let value_after = wait_for(&shell, ".onecalc-home-shell__textarea", move |_| {
        let value = textarea_for_value.value();
        if value.starts_with('=') && value.chars().count() > 3 {
            Some(value)
        } else {
            None
        }
    })
    .await
    .expect("textarea spliced after Tab");
    assert!(value_after.starts_with('='));

    let popup_gone = wait_for(&shell, ".onecalc-home-shell__editor-frame", |_| {
        if shell.select(".onecalc-completion-popup").is_none() {
            Some(())
        } else {
            None
        }
    })
    .await;
    assert!(popup_gone.is_some(), "popup should be hidden after Tab");

    shell.tear_down();
}

/// Enter behaves identically to Tab for popup acceptance. Mirror
/// invariant so a future regression doesn't accidentally diverge the
/// two key handlers.
#[wasm_bindgen_test(async)]
async fn enter_accepts_selected_proposal_and_closes_popup() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    dispatch_keydown(&textarea, "Enter");

    let textarea_for_value = textarea.clone();
    let value_after = wait_for(&shell, ".onecalc-home-shell__textarea", move |_| {
        let value = textarea_for_value.value();
        if value.starts_with('=') && value.chars().count() > 3 {
            Some(value)
        } else {
            None
        }
    })
    .await;
    assert!(value_after.is_some(), "Enter should splice like Tab");

    shell.tear_down();
}

/// Escape dismisses the popup WITHOUT changing the textarea text.
/// Pinned: the popup is gone, the partial input the user typed is
/// preserved.
#[wasm_bindgen_test(async)]
async fn escape_dismisses_popup_without_changing_text() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;
    let value_before = textarea.value();

    dispatch_keydown(&textarea, "Escape");

    let popup_gone = wait_for(&shell, ".onecalc-home-shell__editor-frame", |_| {
        if shell.select(".onecalc-completion-popup").is_none() {
            Some(())
        } else {
            None
        }
    })
    .await;
    assert!(popup_gone.is_some(), "popup should be hidden after Escape");
    assert_eq!(
        textarea.value(),
        value_before,
        "Escape must not change the textarea text",
    );

    shell.tear_down();
}

/// When the popup is Hidden, ArrowLeft / ArrowRight do NOT trigger
/// any popup-state mutation. Pinned by checking that the editor
/// frame's `data-measure-tick` (which advances on every input
/// dispatch through the bridge) is unchanged after a key-only
/// dispatch — i.e. our keydown handler did not fire any
/// state-mutating callback.
#[wasm_bindgen_test(async)]
async fn arrow_keys_when_popup_hidden_do_not_mutate_state() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    // Dispatch some text WITHOUT triggering the popup. A bare `=` is
    // not a function-name prefix; the bridge returns no useful-prefix
    // proposals, so the popup stays Hidden.
    dispatch_input(&textarea, "=");

    let _ = wait_for(&shell, ".onecalc-home-shell__editor-frame", |el| {
        el.get_attribute("data-measure-tick")
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|n| *n >= 1)
    })
    .await;
    let tick_before = shell
        .select(".onecalc-home-shell__editor-frame")
        .and_then(|el| el.get_attribute("data-measure-tick"))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Verify popup is NOT mounted before dispatching the arrow key.
    assert!(
        shell.select(".onecalc-completion-popup").is_none(),
        "precondition: popup must be Hidden for this invariant",
    );

    dispatch_keydown(&textarea, "ArrowLeft");
    dispatch_keydown(&textarea, "ArrowRight");

    let tick_after = shell
        .select(".onecalc-home-shell__editor-frame")
        .and_then(|el| el.get_attribute("data-measure-tick"))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    assert_eq!(
        tick_before, tick_after,
        "popup-Hidden arrow keys must not trigger any reducer round-trip; \
         measure-tick changing implies a state mutation slipped through",
    );

    shell.tear_down();
}

/// Focus-out on the textarea dismisses the popup so it doesn't sit
/// stale on an unfocused editor.
#[wasm_bindgen_test(async)]
async fn focusout_dismisses_open_popup() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    dispatch_focusout(&textarea);

    let popup_gone = wait_for(&shell, ".onecalc-home-shell__editor-frame", |_| {
        if shell.select(".onecalc-completion-popup").is_none() {
            Some(())
        } else {
            None
        }
    })
    .await;
    assert!(
        popup_gone.is_some(),
        "popup should be Hidden after focusout",
    );

    shell.tear_down();
}

/// REPRODUCTION: after a popup acceptance the textarea's caret should
/// land at the end of the inserted text. Today the reducer updates the
/// FormulaSpaceState's caret offset, but the DOM textarea's
/// `selectionStart` is never written — so visually the caret stays
/// wherever it was before the user pressed Tab.
#[wasm_bindgen_test(async)]
async fn caret_lands_at_end_of_inserted_text_after_keyboard_acceptance() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    // Type `=SU` (3 chars). selectionStart is 3 right after dispatch.
    dispatch_input(&textarea, "=SU");
    let selection_before_accept = textarea.selection_start().ok().flatten();
    assert_eq!(
        selection_before_accept,
        Some(3),
        "precondition: caret at offset 3 after typing `=SU`",
    );

    // Wait for popup to mount; record the value before acceptance.
    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;
    let value_before = textarea.value();
    assert_eq!(value_before, "=SU");

    // Press Tab to accept the selected (first) proposal.
    dispatch_keydown(&textarea, "Tab");

    // Wait for the textarea's value to differ from `=SU` (acceptance
    // splice applied through the synthetic input event).
    let textarea_for_value = textarea.clone();
    let value_after = wait_for(&shell, ".onecalc-home-shell__textarea", move |_| {
        let value = textarea_for_value.value();
        if value != "=SU" && value.starts_with('=') && value.chars().count() > 3 {
            Some(value)
        } else {
            None
        }
    })
    .await
    .expect("textarea value updated after Tab acceptance");

    let expected_caret = value_after.chars().count() as u32;
    let actual_caret = textarea
        .selection_start()
        .ok()
        .flatten()
        .expect("selectionStart readable");

    assert_eq!(
        actual_caret, expected_caret,
        "post-acceptance caret should land at the end of `{value_after}` \
         (expected offset {expected_caret}); actual selectionStart = {actual_caret}. \
         If this fails the textarea's DOM selection is not following the reducer's \
         caret update — the home shell's acceptance flow needs to call \
         `textarea.set_selection_range(new_caret, new_caret)` after splicing the value.",
    );

    shell.tear_down();
}

/// REPRODUCTION (real bug found): the syntax-coloring overlay only
/// renders `token.text` for each token, NOT the surrounding trivia
/// (whitespace, comments). For an input like `= SUM` the upstream
/// editor splits this into:
///   - `=`   token (span 0..1)
///   - ` `   whitespace trivia between the tokens
///   - `SUM` token (span 2..5)
/// `syntax_runs_from_snapshot` (in `ui/editor/render_projection.rs`)
/// emits only the two tokens, producing "=SUM" — 4 visible glyphs
/// where the textarea has 5 characters. The caret renders at offset 5
/// in the textarea (which is at the trailing edge of "M" if the
/// overlay had 5 chars, but at column 5 of an overlay that only has
/// 4 visible glyphs — i.e. one glyph past the end of the visible
/// coloured text).
///
/// This invariant pins the contract: the syntax overlay's combined
/// text must exactly match `textarea.value`, character-for-character.
#[wasm_bindgen_test(async)]
async fn syntax_overlay_text_must_match_textarea_value_exactly() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "= SUM");

    // Wait for the bridge round-trip so the overlay is populated.
    let _ = super::scaffold::flush_microtasks(15).await;

    let textarea_value = textarea.value();
    assert_eq!(textarea_value, "= SUM");

    let overlay_element = shell
        .select(".onecalc-home-shell__editor-overlay")
        .expect("syntax overlay mounted");
    let overlay_text = overlay_element.text_content().unwrap_or_default();
    // The overlay appends a trailing newline so its line-box has
    // height even when the last line is empty; strip exactly that
    // one trailing `\n` (not whitespace in general — we want to
    // catch missing-space bugs, which this test exists to detect).
    let overlay_text_stripped = overlay_text
        .strip_suffix('\n')
        .map(|s| s.to_string())
        .unwrap_or(overlay_text);

    assert_eq!(
        overlay_text_stripped,
        textarea_value,
        "syntax overlay text must equal textarea.value character-for-character. \
         Mismatch causes caret position to drift visually away from the textarea \
         content at any offset past a missing trivia run. textarea_value = \
         {textarea_value:?} ({} chars), overlay_text = {overlay_text_stripped:?} \
         ({} chars).",
        textarea_value.chars().count(),
        overlay_text_stripped.chars().count(),
    );

    shell.tear_down();
}

/// REPRODUCTION (user-reported "caret offset from insertion point"):
/// the bug is that after a popup acceptance the home shell does NOT
/// explicitly synchronise the textarea's `selectionStart` to the
/// acceptance's `new_caret_offset`. It relies on whatever the browser
/// happens to do when `textarea.value` is rewritten by Leptos's
/// `prop:value` reactivity. Headless Edge moves the caret to the end
/// of the new value (which usually masks the bug); other browsers
/// preserve the prior `selectionStart` (clamped to the new length),
/// which lands the caret in the middle of the just-inserted token.
///
/// We force the bug deterministically by:
/// 1. Typing `=SU` (popup opens, caret at offset 3).
/// 2. Programmatically moving `selectionStart` to 1 — simulates a
///    browser that, on the post-acceptance `value` change, leaves
///    the cursor at its prior offset rather than auto-moving to end.
/// 3. Pressing Tab to accept.
///
/// After acceptance, the home shell's reducer puts state-side caret
/// at `replacement_span.start + insert_text.chars().count()` (e.g.
/// `1 + len("SUBSTITUTE") = 11`). The DOM's `selectionStart` should
/// match. Without an explicit `textarea.set_selection_range(...)`
/// call after the splice, the DOM caret stays at 1 — which is
/// exactly the user's report: typing the next character lands
/// right after the `=` instead of after the inserted function name.
#[wasm_bindgen_test(async)]
async fn caret_dom_selection_is_synced_to_acceptance_offset_after_tab() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    // Clobber selection to simulate the cross-browser case where
    // textarea.value rewrites do NOT auto-move the caret to the
    // value's end. After this, the DOM selectionStart is 1 even
    // though we just typed three characters.
    textarea
        .set_selection_range(1, 1)
        .expect("set selection range");
    assert_eq!(
        textarea.selection_start().ok().flatten(),
        Some(1),
        "precondition: caret pinned at offset 1 before Tab",
    );

    dispatch_keydown(&textarea, "Tab");

    let textarea_for_value = textarea.clone();
    let value_after = wait_for(&shell, ".onecalc-home-shell__textarea", move |_| {
        let value = textarea_for_value.value();
        if value != "=SU" && value.starts_with('=') && value.chars().count() > 3 {
            Some(value)
        } else {
            None
        }
    })
    .await
    .expect("value spliced after Tab");

    let expected_caret = value_after.chars().count() as u32;
    let actual_caret = textarea
        .selection_start()
        .ok()
        .flatten()
        .expect("selectionStart readable");

    assert_eq!(
        actual_caret, expected_caret,
        "DOM caret should be explicitly synced to the acceptance's \
         new_caret_offset (`{expected_caret}` for `{value_after}`); \
         actual = {actual_caret}. The fix is to call \
         `textarea.set_selection_range(new_caret, new_caret)` after \
         the splice in the home shell's apply_acceptance.",
    );

    shell.tear_down();
}

/// REPRODUCTION (user-reported): type `= SUM` (note the space between
/// `=` and `SUM`), then press Tab. The caret should land at the end of
/// the spliced result. The user-typed sequence puts the popup-trigger
/// prefix `SUM` starting at offset 2 (after `= `), and the bridge's
/// proposals will use a `replacement_span` starting at offset 2.
///
/// The acceptance reducer computes `new_caret_offset = span.start +
/// insert_text.chars().count()` = `2 + 10` for `SUBSTITUTE`. If
/// anything in the splice / state / DOM-sync chain disagrees with that
/// arithmetic — for example using `caret_offset + insert_len` instead
/// of `span.start + insert_len`, or applying the splice to a stale
/// raw_text snapshot — the textarea will end up with the caret offset
/// from the end of the inserted token.
#[wasm_bindgen_test(async)]
async fn caret_lands_at_end_when_prefix_starts_inside_text_not_at_offset_zero() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    // Build "= SUM" via successive input events to mirror the
    // user's interactive sequence as closely as possible (each
    // dispatch is a separate bridge round-trip + popup sync).
    dispatch_input(&textarea, "=");
    dispatch_input(&textarea, "= ");
    dispatch_input(&textarea, "= S");
    dispatch_input(&textarea, "= SU");
    dispatch_input(&textarea, "= SUM");

    // Wait for the popup to mount with proposals matching `SUM`.
    let popup_count = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;
    assert!(
        popup_count.is_some_and(|n| n >= 1),
        "popup must be Open before Tab — typing `= SUM` should produce SUM-family proposals; \
         got popup count = {popup_count:?}",
    );

    let value_before = textarea.value();
    assert_eq!(value_before, "= SUM");
    assert_eq!(
        textarea.selection_start().ok().flatten(),
        Some(5),
        "precondition: caret at offset 5 after typing `= SUM`",
    );

    // Snapshot popup state BEFORE Tab so we can confirm it's actually
    // mounted. Read each item's proposal_id + data-selected to see the
    // shape of what acceptance is supposed to apply.
    let popup_before_attrs = shell.select(".onecalc-completion-popup").map(|el| {
        (
            el.get_attribute("data-item-count").unwrap_or_default(),
            el.get_attribute("data-selected-index").unwrap_or_default(),
        )
    });
    let first_item_id = shell
        .select(".onecalc-completion-popup__item")
        .and_then(|el| el.get_attribute("data-proposal-id"));
    assert!(
        popup_before_attrs.is_some(),
        "diagnostic: popup mounted before Tab? attrs = {popup_before_attrs:?}",
    );
    assert!(
        first_item_id.is_some(),
        "diagnostic: first item present before Tab? id = {first_item_id:?}",
    );

    dispatch_keydown(&textarea, "Tab");
    super::scaffold::flush_microtasks(15).await;

    // After Tab — accepting SUM in `= SUM` SPLICES nothing visible (the
    // proposal's insert_text equals the existing 'SUM') so the textarea
    // value stays "= SUM". The interesting thing is what the DOM
    // selection actually is. Then dispatch ONE more character to
    // reveal where subsequent typing lands — this is the user-visible
    // proxy for "caret is offset".
    let value_mid = textarea.value();
    let selection_mid = textarea.selection_start().ok().flatten();
    let value_after_extra = {
        // Type "(" — emulates the natural follow-up after accepting a
        // function name. The browser's textarea normally inserts the
        // character at selectionStart and advances the caret.
        let current = textarea.value();
        let caret = textarea
            .selection_start()
            .ok()
            .flatten()
            .unwrap_or(current.chars().count() as u32) as usize;
        let chars: Vec<char> = current.chars().collect();
        let mut next: String = chars[..caret].iter().collect();
        next.push('(');
        let trailing: String = chars[caret..].iter().collect();
        next.push_str(&trailing);
        dispatch_input(&textarea, &next);
        next
    };
    let value_final = textarea.value();
    let selection_final = textarea.selection_start().ok().flatten();

    // The user-reported bug: after Tab acceptance, typing the next
    // character lands at the wrong offset. The expected "(" lands at
    // the END of the function name; if the caret drifted backward
    // (e.g. to the post-acceptance state's caret offset 5 from the
    // splice arithmetic, while the DOM held a different number), the
    // "(" splice from `dispatch_input` would put it elsewhere.
    let expected_after_extra = "= SUM(";
    assert_eq!(
        value_final, expected_after_extra,
        "expected `{expected_after_extra}` after Tab + `(` keystroke; \
         got value_final = {value_final:?}. value_mid = {value_mid:?}, \
         selection_mid = {selection_mid:?}, simulated_typed_value = \
         {value_after_extra:?}, selection_final = {selection_final:?}",
    );

    shell.tear_down();
}

/// REPRODUCTION (mouse path): same caret invariant but driven through a
/// mousedown click on a popup row instead of Tab. Pins that the click
/// path has the same caret-sync contract as the keyboard path so a
/// future fix doesn't accidentally leave one of them unrepaired.
#[wasm_bindgen_test(async)]
async fn caret_lands_at_end_of_inserted_text_after_mouse_acceptance() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    // Click the first row.
    let first = shell
        .select(".onecalc-completion-popup__item")
        .expect("first item present");
    let mousedown_init = web_sys::MouseEventInit::new();
    mousedown_init.set_bubbles(true);
    mousedown_init.set_cancelable(true);
    let mousedown =
        web_sys::MouseEvent::new_with_mouse_event_init_dict("mousedown", &mousedown_init)
            .expect("mousedown event");
    first
        .dispatch_event(&mousedown)
        .expect("dispatch mousedown");

    let textarea_for_value = textarea.clone();
    let value_after = wait_for(&shell, ".onecalc-home-shell__textarea", move |_| {
        let value = textarea_for_value.value();
        if value != "=SU" && value.starts_with('=') && value.chars().count() > 3 {
            Some(value)
        } else {
            None
        }
    })
    .await
    .expect("textarea value updated after mouse acceptance");

    let expected_caret = value_after.chars().count() as u32;
    let actual_caret = textarea
        .selection_start()
        .ok()
        .flatten()
        .expect("selectionStart readable");

    assert_eq!(
        actual_caret, expected_caret,
        "post-acceptance caret should land at end of `{value_after}` \
         (offset {expected_caret}); actual = {actual_caret}",
    );

    shell.tear_down();
}

/// Suppression-after-accept: a keyboard acceptance (Tab) closes the
/// popup, and the synthetic input event that propagates the new
/// textarea value through the bridge does NOT re-open the popup
/// even though the bridge's proposal list now matches the
/// just-inserted function name. Pinned at the DOM level so a
/// regression in the suppression flag is caught here.
#[wasm_bindgen_test(async)]
async fn suppression_after_accept_keeps_popup_hidden_through_bridge_refresh() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SU");

    let _ = wait_for(&shell, ".onecalc-completion-popup", |el| {
        el.get_attribute("data-item-count")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n >= 1)
    })
    .await;

    dispatch_keydown(&textarea, "Tab");

    // Popup gone within reasonable settle window.
    let popup_gone = wait_for(&shell, ".onecalc-home-shell__editor-frame", |_| {
        if shell.select(".onecalc-completion-popup").is_none() {
            Some(())
        } else {
            None
        }
    })
    .await;
    assert!(popup_gone.is_some());

    // Crucially: the popup STAYS gone across a few extra microtask
    // ticks (the bridge refresh that the synthetic input dispatched
    // would have re-opened it WITHOUT the suppression flag).
    for _ in 0..10 {
        super::scaffold::next_microtask().await;
    }
    assert!(
        shell.select(".onecalc-completion-popup").is_none(),
        "suppression must keep popup hidden across post-accept bridge refresh",
    );

    let _ = popup_item_count;
    let _ = wait_for_text;
    shell.tear_down();
}

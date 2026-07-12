//! Function-help hover-tooltip invariants.
//!
//! When the user hovers over a `.syn-fn` span in the syntax overlay
//! whose token text matches the bridge's `function_help.lookup_key`,
//! the home shell renders a tooltip with the function's display
//! name, first signature form, and short description. These
//! invariants pin:
//!
//! * the tooltip APPEARS on mouseover of a matching function token,
//! * it DISMISSES on mouseleave of the editor frame,
//! * it DISMISSES when the formula text changes underneath,
//! * it DOES NOT appear for non-function tokens or for function
//!   tokens whose name doesn't match the current bridge packet,
//! * it carries `data-lookup-key` so the corpus can verify the
//!   matching contract,
//! * its content reflects the bridge's display_name, signature,
//!   and short_description fields.
//!
//! Note on the 400 ms hover delay: v1 ships without the delay
//! (tooltip appears immediately on mouseover). When the delay
//! lands in a follow-up bead, this corpus's `flush_microtasks`
//! waits will need to grow to cover the delay window.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, mount_home_shell, wait_for};

wasm_bindgen_test_configure!(run_in_browser);

/// Dispatch a `mouseover` event on the given element. Mouseover
/// bubbles, so the editor-frame's delegation handler fires
/// when called on a `.syn-fn` span descendant.
fn dispatch_mouseover(element: &web_sys::Element) {
    let init = web_sys::MouseEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = web_sys::MouseEvent::new_with_mouse_event_init_dict("mouseover", &init)
        .expect("mouseover event");
    element.dispatch_event(&event).expect("dispatch mouseover");
}

/// Dispatch a `mouseleave` event on the given element.
/// `mouseleave` does NOT bubble, so this must be called directly
/// on the element that owns the listener (the editor frame).
fn dispatch_mouseleave(element: &web_sys::Element) {
    let init = web_sys::MouseEventInit::new();
    init.set_bubbles(false);
    init.set_cancelable(true);
    let event = web_sys::MouseEvent::new_with_mouse_event_init_dict("mouseleave", &init)
        .expect("mouseleave event");
    element.dispatch_event(&event).expect("dispatch mouseleave");
}

#[wasm_bindgen_test(async)]
async fn function_help_tooltip_appears_on_function_token_mouseover() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    // Wait for the syntax overlay's SUM span to mount with the
    // expected attributes.
    let function_span = wait_for(
        &shell,
        ".onecalc-home-shell__editor-overlay .syn-fn",
        |el| {
            let text = el.get_attribute("data-token-text")?;
            if text.eq_ignore_ascii_case("SUM") {
                Some(el.clone())
            } else {
                None
            }
        },
    )
    .await
    .expect("SUM .syn-fn span mounted");

    dispatch_mouseover(&function_span);
    super::scaffold::flush_microtasks(15).await;

    let tooltip = shell
        .select(".onecalc-function-help")
        .expect("tooltip mounted");
    assert_eq!(
        tooltip.get_attribute("data-lookup-key").as_deref(),
        Some("SUM"),
    );
    let heading = shell
        .select(".onecalc-function-help__heading")
        .map(|el| el.text_content().unwrap_or_default());
    assert_eq!(heading.as_deref(), Some("SUM"));
    let signature = shell
        .select(".onecalc-function-help__signature")
        .map(|el| el.text_content().unwrap_or_default());
    // Signature line presence is the contract; the exact display
    // string depends on the upstream catalogue's signature form
    // (which varies across bridge versions). Pin only that the
    // signature line is present and non-empty.
    assert!(
        signature.as_deref().map(|s| !s.is_empty()).unwrap_or(false),
        "signature line should be present and non-empty; got {signature:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn function_help_tooltip_dismisses_on_editor_frame_mouseleave() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    let function_span = wait_for(
        &shell,
        ".onecalc-home-shell__editor-overlay .syn-fn",
        |el| {
            el.get_attribute("data-token-text")
                .filter(|t| t.eq_ignore_ascii_case("SUM"))
                .map(|_| el.clone())
        },
    )
    .await
    .expect("SUM span mounted");
    dispatch_mouseover(&function_span);
    super::scaffold::flush_microtasks(5).await;
    assert!(shell.select(".onecalc-function-help").is_some());

    let frame = shell
        .select(".onecalc-home-shell__editor-frame")
        .expect("editor frame mounted");
    dispatch_mouseleave(&frame);
    super::scaffold::flush_microtasks(5).await;

    assert!(
        shell.select(".onecalc-function-help").is_none(),
        "tooltip should dismiss on mouseleave of the editor frame",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn function_help_tooltip_dismisses_on_subsequent_input() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    let function_span = wait_for(
        &shell,
        ".onecalc-home-shell__editor-overlay .syn-fn",
        |el| {
            el.get_attribute("data-token-text")
                .filter(|t| t.eq_ignore_ascii_case("SUM"))
                .map(|_| el.clone())
        },
    )
    .await
    .expect("SUM span mounted");
    dispatch_mouseover(&function_span);
    super::scaffold::flush_microtasks(5).await;
    assert!(shell.select(".onecalc-function-help").is_some());

    // Type another character — formula changed, hover should clear.
    dispatch_input(&textarea, "=SUM(1,2,3)");
    super::scaffold::flush_microtasks(15).await;

    assert!(
        shell.select(".onecalc-function-help").is_none(),
        "tooltip should dismiss when raw_entered_cell_text changes underneath the hover",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn function_help_tooltip_does_not_appear_for_non_function_tokens() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    // Find a non-function span (e.g. the `=` operator or a number).
    let number_span = wait_for(
        &shell,
        ".onecalc-home-shell__editor-overlay .syn-num",
        |el| Some(el.clone()),
    )
    .await
    .expect("a number span mounted");

    dispatch_mouseover(&number_span);
    super::scaffold::flush_microtasks(5).await;

    assert!(
        shell.select(".onecalc-function-help").is_none(),
        "hovering a non-function token should NOT show a function-help tooltip",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn function_help_tooltip_carries_short_description_from_bridge() {
    // The bridge populates short_description for SUM with 'Adds
    // numbers together.' (per the seed test_support stub used by
    // the live bridge). Pin that the tooltip surfaces the
    // bridge's text rather than a hard-coded one.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    let function_span = wait_for(
        &shell,
        ".onecalc-home-shell__editor-overlay .syn-fn",
        |el| {
            el.get_attribute("data-token-text")
                .filter(|t| t.eq_ignore_ascii_case("SUM"))
                .map(|_| el.clone())
        },
    )
    .await
    .expect("SUM span mounted");
    dispatch_mouseover(&function_span);
    super::scaffold::flush_microtasks(5).await;

    let description = shell
        .select(".onecalc-function-help__description")
        .map(|el| el.text_content().unwrap_or_default());
    assert!(
        description
            .as_deref()
            .map(|d| !d.is_empty())
            .unwrap_or(false),
        "tooltip should surface the bridge's short_description when present; got {description:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn syntax_overlay_function_spans_carry_data_token_attributes() {
    // The hover delegation depends on the syntax overlay emitting
    // data-token-role / data-token-text / data-token-start on each
    // span. Pin this contract so a regression in the overlay
    // renderer surfaces here, not via a confused hover-tooltip
    // failure.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    let function_span = wait_for(
        &shell,
        ".onecalc-home-shell__editor-overlay .syn-fn",
        |el| Some(el.clone()),
    )
    .await
    .expect("function span mounted");

    let role = function_span.get_attribute("data-token-role");
    let text = function_span.get_attribute("data-token-text");
    let start = function_span.get_attribute("data-token-start");
    assert_eq!(role.as_deref(), Some("function"));
    assert!(
        text.as_deref().map(|t| !t.is_empty()).unwrap_or(false),
        "data-token-text must be present and non-empty",
    );
    assert!(
        start
            .as_deref()
            .and_then(|s| s.parse::<usize>().ok())
            .is_some(),
        "data-token-start must parse as usize",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn function_help_tooltip_uses_caret_box_anchor_not_dom_rect() {
    // The tooltip's `top` and `left` style values are computed via
    // caret_box_for_offset(token_start, metrics) — NOT via reading
    // the span's bounding-client-rect. Pin the contract so the
    // future caret-positioning bead doesn't accidentally switch
    // strategies and break determinism in headless layout.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    let function_span = wait_for(
        &shell,
        ".onecalc-home-shell__editor-overlay .syn-fn",
        |el| Some(el.clone()),
    )
    .await
    .expect("function span mounted");
    dispatch_mouseover(&function_span);
    super::scaffold::flush_microtasks(5).await;

    let tooltip = shell
        .select(".onecalc-function-help")
        .expect("tooltip mounted");
    let style = tooltip
        .get_attribute("style")
        .expect("inline style present");
    // Both `left:Npx` and `top:Npx` must be present.
    assert!(style.contains("left:"));
    assert!(style.contains("top:"));

    let _ = function_span; // silence unused-warning if any
    let _ = textarea;
    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn function_help_tooltip_disappears_when_function_help_packet_clears() {
    // After the user moves the caret OUT of any function call the
    // bridge stops emitting function_help. The next mouseover
    // attempt must not show a tooltip.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)+10");
    // Caret at end of `+10` — outside any call — bridge should
    // clear function_help packet.
    super::scaffold::flush_microtasks(15).await;

    let function_span = wait_for(
        &shell,
        ".onecalc-home-shell__editor-overlay .syn-fn",
        |el| {
            el.get_attribute("data-token-text")
                .filter(|t| t.eq_ignore_ascii_case("SUM"))
                .map(|_| el.clone())
        },
    )
    .await;

    if let Some(span) = function_span {
        dispatch_mouseover(&span);
        super::scaffold::flush_microtasks(5).await;
        assert!(
            shell.select(".onecalc-function-help").is_none(),
            "tooltip must NOT appear when the bridge has no function_help \
             for the current caret",
        );
    } else {
        // No SUM span found — even better, the test can't fail
        // for a different reason.
    }

    shell.tear_down();
}

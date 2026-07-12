//! Browser tests (wasm-bindgen-test, headless) driving the real Leptos
//! components in a live DOM. Modeled on
//! `dnacalc-formula-skin-leptos/tests/browser.rs` and the workspace `.cargo`
//! runner config (geckodriver + Firefox).
//!
//! Two behaviors the native vm tests cannot reach:
//! 1. Typing into the editor emits the `BridgeEvent` sequence
//!    (`TextEdited` -> `CommitRequested` -> `RevertRequested`), and the
//!    typed text arrives byte-for-byte verbatim through the real `on:input`.
//! 2. Completion keyboard navigation (Down/Down/Enter) applies the landed
//!    proposal by id, and Esc closes the popup so Enter then commits instead.
//! 3. Degrade mode renders ZERO token-role classes in the live DOM.

#![cfg(target_arch = "wasm32")]

use std::sync::{Arc, Mutex};

use dnacalc_bridge::{BridgeEvent, FormulaBridge, FormulaBridgeDegrade};
use dnacalc_skin_ir::formula::{
    CompletionItemProjection, CompletionKindProjection, CompletionSurface, FormulaAssistSurface,
    FormulaEditorSurface,
};
use dnacalc_skin_ir::workspace::GridEntryDiagnosticProjection;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

/// Yield to the event loop so Leptos flushes its reactive DOM updates before a
/// DOM assertion reads them (the repo's established browser-test pattern).
async fn next_tick() {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0)
            .unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
}

fn fresh_host() -> web_sys::HtmlElement {
    let host = document()
        .create_element("div")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    document().body().unwrap().append_child(&host).unwrap();
    host
}

fn textarea(host: &web_sys::HtmlElement) -> web_sys::HtmlTextAreaElement {
    host.query_selector("textarea")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .unwrap()
}

/// Dispatch a bubbling `input` event after setting the textarea value — the
/// browser's own typing path.
fn type_text(area: &web_sys::HtmlTextAreaElement, value: &str) {
    area.set_value(value);
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    let event = web_sys::Event::new_with_event_init_dict("input", &init).unwrap();
    area.dispatch_event(&event).unwrap();
}

/// Dispatch a bubbling, cancelable `keydown` for `key` on `target`.
fn press_key(target: &web_sys::EventTarget, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    target.dispatch_event(&event).unwrap();
}

/// Dispatch a bubbling, cancelable Ctrl+`key` `keydown` on `target`.
fn press_ctrl_key(target: &web_sys::EventTarget, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_ctrl_key(true);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    target.dispatch_event(&event).unwrap();
}

fn empty_editor() -> FormulaEditorSurface {
    FormulaEditorSurface {
        source_text: String::new(),
        document_is_fresh: true,
        ..Default::default()
    }
}

fn mount_full(
    editor: FormulaEditorSurface,
    assist: FormulaAssistSurface,
    sink: Arc<Mutex<Vec<BridgeEvent>>>,
) -> web_sys::HtmlElement {
    let host = fresh_host();
    let on_event = Callback::new(move |event| sink.lock().unwrap().push(event));
    leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        view! { <FormulaBridge editor=editor.clone() assist=assist.clone() on_event=on_event /> }
    })
    .forget();
    host
}

// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
fn typing_emits_text_edited_then_commit_then_revert_verbatim() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let host = mount_full(
        empty_editor(),
        FormulaAssistSurface::default(),
        events.clone(),
    );
    let area = textarea(&host);

    // A pasted formula with embedded quotes/equals must arrive byte-for-byte.
    let pasted = "=XLOOKUP(\"a=b\", R[ISO], R[Fx])";
    type_text(&area, pasted);
    press_key(&area, "Enter");
    press_key(&area, "Escape");

    let observed = events.lock().unwrap();
    assert!(observed.len() >= 3, "want >=3 events, got {observed:?}");
    match &observed[0] {
        BridgeEvent::TextEdited { text, .. } => {
            assert_eq!(text, pasted, "typed text must reach the event verbatim");
        }
        other => panic!("first event should be TextEdited, got {other:?}"),
    }
    assert!(
        observed
            .iter()
            .any(|e| matches!(e, BridgeEvent::CommitRequested)),
        "Enter must emit CommitRequested"
    );
    assert!(
        observed
            .iter()
            .any(|e| matches!(e, BridgeEvent::RevertRequested)),
        "Escape must emit RevertRequested"
    );
}

fn completion_assist() -> FormulaAssistSurface {
    let items = ["p0", "p1", "p2"]
        .iter()
        .map(|id| CompletionItemProjection {
            proposal_id: (*id).to_string(),
            display_text: format!("fn-{id}"),
            kind: CompletionKindProjection::Function,
            documentation_ref: None,
        })
        .collect();
    FormulaAssistSurface {
        completion: Some(CompletionSurface {
            anchor_left_px: 10,
            anchor_top_px: 20,
            line_height_px: 18,
            selected_index: 0,
            items,
        }),
        ..Default::default()
    }
}

#[wasm_bindgen_test]
fn completion_keyboard_navigation_applies_the_landed_proposal() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let host = mount_full(empty_editor(), completion_assist(), events.clone());
    let area = textarea(&host);

    // Popup is open (items present). Down, Down lands on index 2 (p2).
    assert!(
        host.query_selector(".dna-bridge__completion")
            .unwrap()
            .is_some(),
        "completion popup should be open on mount"
    );
    press_key(&area, "ArrowDown");
    press_key(&area, "ArrowDown");
    press_key(&area, "Enter");

    let observed = events.lock().unwrap();
    assert_eq!(
        observed.last(),
        Some(&BridgeEvent::CompletionApplied {
            proposal_id: "p2".to_string()
        }),
        "Down/Down/Enter must apply p2, got {observed:?}"
    );
}

#[wasm_bindgen_test]
async fn escape_closes_completion_then_enter_commits() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let host = mount_full(empty_editor(), completion_assist(), events.clone());
    let area = textarea(&host);

    // Esc closes the popup (overlay-first grammar) and emits nothing.
    press_key(&area, "Escape");
    next_tick().await;
    assert!(
        host.query_selector(".dna-bridge__completion")
            .unwrap()
            .is_none(),
        "Esc should close the completion popup"
    );
    // With the popup closed, Enter now commits rather than applying.
    press_key(&area, "Enter");
    let observed = events.lock().unwrap();
    assert!(
        observed
            .iter()
            .any(|e| matches!(e, BridgeEvent::CommitRequested)),
        "Enter after closing popup must emit CommitRequested, got {observed:?}"
    );
    assert!(
        !observed
            .iter()
            .any(|e| matches!(e, BridgeEvent::CompletionApplied { .. })),
        "no proposal should be applied after Esc"
    );
}

#[wasm_bindgen_test]
async fn degrade_mode_renders_zero_token_role_classes() {
    let host = fresh_host();
    let events: Arc<Mutex<Vec<BridgeEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let on_event = Callback::new(move |event| sink.lock().unwrap().push(event));
    let rejections = vec![GridEntryDiagnosticProjection {
        message: "circular reference".to_string(),
        span: Some((1, 4)),
    }];
    leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        view! {
            <FormulaBridgeDegrade
                text="=ABC+1".to_string()
                rejections=rejections.clone()
                on_event=on_event
            />
        }
    })
    .forget();
    next_tick().await;

    // No *element* may carry a token-role class. Query the class attribute
    // (an attribute-substring selector matches element classes, never the
    // inlined <style> text — the earlier false positive).
    let role_elements = host.query_selector_all("[class*=\"--role-\"]").unwrap();
    assert_eq!(
        role_elements.length(),
        0,
        "degrade mode must render zero token-role class elements"
    );
    // But it IS a degrade bridge, and the rejection underline (Error) shows.
    assert!(
        host.query_selector("[data-mode=\"degrade\"]")
            .unwrap()
            .is_some()
    );
    assert!(
        host.query_selector(".dna-bridge__seg--diag-error")
            .unwrap()
            .is_some()
    );

    // Typing still emits verbatim TextEdited through the degrade path.
    let area = textarea(&host);
    type_text(&area, "=NOT@PARSED=");
    let observed = events.lock().unwrap();
    assert!(
        matches!(observed.first(), Some(BridgeEvent::TextEdited { text, .. }) if text == "=NOT@PARSED="),
        "degrade typing must pass through verbatim, got {observed:?}"
    );
}

/// Mount `FormulaBridge` nested one level inside a stand-in "shell root" div
/// carrying its own `on:keydown` — modeling the composed Bench app, where
/// the editor lives inside `.dna-shell`'s DOM subtree and the shell's real
/// keydown dispatcher is an ancestor listener that only ever observes
/// whatever the editor lets bubble.
fn mount_inside_shell_stub(
    editor: FormulaEditorSurface,
    assist: FormulaAssistSurface,
    sink: Arc<Mutex<Vec<BridgeEvent>>>,
    bubbled: Arc<Mutex<Vec<String>>>,
) -> web_sys::HtmlElement {
    let host = fresh_host();
    let on_event = Callback::new(move |event| sink.lock().unwrap().push(event));
    leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        view! {
            <div
                data-testid="shell-stub"
                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                    bubbled.lock().unwrap().push(ev.key());
                }
            >
                <FormulaBridge editor=editor.clone() assist=assist.clone() on_event=on_event />
            </div>
        }
    })
    .forget();
    host
}

#[wasm_bindgen_test]
fn f9_and_ctrl_k_bubble_past_the_editor_to_an_ancestor_shell_handler() {
    // Bead dtc-1tk.1 / H1b: `on_keydown` used to call `ev.stop_propagation()`
    // unconditionally for every keydown, so F9 (Recalculate) and Ctrl+K
    // (command deck) — chords the editor never handles — could never reach
    // the shell's own `.dna-shell` keydown dispatcher in the composed Bench
    // app, defeating SHELL_SPEC §5's tested F-key exemption ("F9 must work
    // from inside edit buffers"). This mounts the editor nested inside a
    // stand-in shell root with its own ancestor `on:keydown`, exactly
    // modeling that composition, and proves un-consumed chords now bubble.
    let events = Arc::new(Mutex::new(Vec::new()));
    let bubbled = Arc::new(Mutex::new(Vec::new()));
    let host = mount_inside_shell_stub(
        empty_editor(),
        FormulaAssistSurface::default(),
        events.clone(),
        bubbled.clone(),
    );
    let area = textarea(&host);

    press_key(&area, "F9");
    assert_eq!(
        bubbled.lock().unwrap().as_slice(),
        ["F9"],
        "F9 must bubble past the editor to the shell (SHELL_SPEC §5 F-key exemption)"
    );

    press_ctrl_key(&area, "k");
    assert_eq!(
        bubbled.lock().unwrap().as_slice(),
        ["F9", "k"],
        "Ctrl+K must bubble past the editor to reach the shell's command-deck chord"
    );

    // Contrast: Escape IS consumed by the editor (host-side exact revert)
    // and must NOT also bubble to the shell in the same keystroke — see the
    // propagation-precedence note on `on_keydown` in dnacalc-bridge/src/editor.rs.
    press_key(&area, "Escape");
    assert_eq!(
        bubbled.lock().unwrap().as_slice(),
        ["F9", "k"],
        "Escape is consumed locally by the editor's revert and must not bubble"
    );
    let observed = events.lock().unwrap();
    assert!(
        observed
            .iter()
            .any(|e| matches!(e, BridgeEvent::RevertRequested)),
        "the editor must still have handled Escape itself, got {observed:?}"
    );
}

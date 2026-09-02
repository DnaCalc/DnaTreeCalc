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

use dnacalc_bridge::{BridgeEvent, CommitAdvance, FormulaBridge, FormulaBridgeDegrade};
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

/// Dispatch a bubbling, cancelable Ctrl+`key` `keydown` on `target`. Returns
/// the DOM dispatch result: `true` if no listener called `preventDefault()`,
/// `false` if one did — the direct way to assert a consumed key still let
/// the browser's own default action (e.g. native textarea undo) survive.
fn press_ctrl_key(target: &web_sys::EventTarget, key: &str) -> bool {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_ctrl_key(true);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    target.dispatch_event(&event).unwrap()
}

/// Dispatch a bubbling, cancelable Ctrl+Shift+`key` `keydown` on `target`
/// (e.g. Ctrl+Shift+Z, the redo alternate). Returns the DOM dispatch result
/// (see `press_ctrl_key`).
fn press_ctrl_shift_key(target: &web_sys::EventTarget, key: &str) -> bool {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_ctrl_key(true);
    init.set_shift_key(true);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    target.dispatch_event(&event).unwrap()
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
            .any(|e| matches!(e, BridgeEvent::CommitRequested { .. })),
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
            .any(|e| matches!(e, BridgeEvent::CommitRequested { .. })),
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

/// Mount a bare `FormulaBridgeDegrade` with a chosen `commit_on_tab`, collecting
/// its events into `sink` (dtc-dzky test support).
fn mount_degrade(commit_on_tab: bool, sink: Arc<Mutex<Vec<BridgeEvent>>>) -> web_sys::HtmlElement {
    let host = fresh_host();
    let on_event = Callback::new(move |event| sink.lock().unwrap().push(event));
    leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        view! { <FormulaBridgeDegrade commit_on_tab=commit_on_tab on_event=on_event /> }
    })
    .forget();
    host
}

/// dtc-dzky: the degrade editor commits with a DIRECTION — Enter advances Down,
/// and Tab advances Right, but Tab only where the host opted in (`commit_on_tab`,
/// a grid). Without opt-in, Tab is left to the browser (emits no commit), so a
/// Notebook block / the single-formula Bench slot is unchanged.
#[wasm_bindgen_test]
fn degrade_commit_carries_enter_down_and_opt_in_tab_right() {
    // Opted in (the Sheet grid): Enter → Down, Tab → Right.
    let events: Arc<Mutex<Vec<BridgeEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let host = mount_degrade(true, events.clone());
    let area = textarea(&host);

    press_key(&area, "Enter");
    assert_eq!(
        events.lock().unwrap().last(),
        Some(&BridgeEvent::CommitRequested {
            advance: CommitAdvance::Down
        }),
        "Enter commits with the Down advance"
    );
    press_key(&area, "Tab");
    assert_eq!(
        events.lock().unwrap().last(),
        Some(&BridgeEvent::CommitRequested {
            advance: CommitAdvance::Right
        }),
        "Tab (opted in) commits with the Right advance"
    );

    // NOT opted in (the default): Tab emits no commit — it stays a browser focus
    // move, so Notebook/Bench keep their behavior; Enter still commits (Down).
    let events2: Arc<Mutex<Vec<BridgeEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let host2 = mount_degrade(false, events2.clone());
    let area2 = textarea(&host2);
    press_key(&area2, "Tab");
    assert!(
        !events2
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, BridgeEvent::CommitRequested { .. })),
        "Tab must NOT commit without opt-in, got {:?}",
        events2.lock().unwrap()
    );
    press_key(&area2, "Enter");
    assert_eq!(
        events2.lock().unwrap().last(),
        Some(&BridgeEvent::CommitRequested {
            advance: CommitAdvance::Down
        }),
        "Enter commits (Down) even without the tab opt-in"
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

/// Mount `FormulaBridgeDegrade` nested inside the same stand-in shell root as
/// `mount_inside_shell_stub` — modeling the composed Calc app, where the Sheet
/// stage's overlay editor IS this degrade bridge and lives inside `.dna-shell`.
/// `text` is the mount seed and `committed` the host's committed text where
/// the Sheet stage supplies one (dtc-j7n8.25).
fn mount_degrade_inside_shell_stub(
    commit_on_tab: bool,
    text: &str,
    committed: Option<&str>,
    sink: Arc<Mutex<Vec<BridgeEvent>>>,
    bubbled: Arc<Mutex<Vec<String>>>,
) -> web_sys::HtmlElement {
    let host = fresh_host();
    let on_event = Callback::new(move |event| sink.lock().unwrap().push(event));
    let text = text.to_string();
    let committed = committed.map(str::to_string);
    leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        view! {
            <div
                data-testid="shell-stub"
                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                    bubbled.lock().unwrap().push(ev.key());
                }
            >
                <FormulaBridgeDegrade
                    text=text
                    committed=committed
                    commit_on_tab=commit_on_tab
                    on_event=on_event
                />
            </div>
        }
    })
    .forget();
    host
}

#[wasm_bindgen_test]
fn ctrl_s_bubbles_past_the_degrade_editor_to_an_ancestor_shell_handler() {
    // Bead dtc-j7n8.25: Ctrl+S did nothing while keyboard focus was inside the
    // Sheet stage. The stage's overlay editor is THIS degrade bridge, and its
    // `on_keydown` called `ev.stop_propagation()` on every keydown before
    // matching — so once dtc-j7n8.26 handed the editor focus, Ctrl+S (the
    // shell's Save verb), Ctrl+O, Ctrl+K and F9 typed while editing a cell
    // never reached the shell's `.dna-shell` keydown pipeline. Same law as the
    // full editor's `f9_and_ctrl_k_bubble_past_the_editor_...` test above:
    // only what the editor consumes may stop.
    let events = Arc::new(Mutex::new(Vec::new()));
    let bubbled = Arc::new(Mutex::new(Vec::new()));
    let host = mount_degrade_inside_shell_stub(true, "", None, events.clone(), bubbled.clone());
    let area = textarea(&host);

    let default_allowed = press_ctrl_key(&area, "s");
    assert_eq!(
        bubbled.lock().unwrap().as_slice(),
        ["s"],
        "Ctrl+S must bubble past the degrade editor to the shell's Save verb"
    );
    assert!(
        default_allowed,
        "the degrade editor must not preventDefault a chord it does not own (the shell does)"
    );
    press_ctrl_key(&area, "o");
    press_key(&area, "F9");
    assert_eq!(
        bubbled.lock().unwrap().as_slice(),
        ["s", "o", "F9"],
        "Ctrl+O and F9 bubble too (SHELL_SPEC §5 exemption class)"
    );

    // Contrast: Enter / Escape ARE the editor's — consumed, never bubbled.
    press_key(&area, "Enter");
    press_key(&area, "Escape");
    assert_eq!(
        bubbled.lock().unwrap().as_slice(),
        ["s", "o", "F9"],
        "Enter (commit) and Escape (revert) are consumed locally and must not bubble"
    );
    let observed = events.lock().unwrap();
    assert!(
        observed.iter().any(|e| matches!(
            e,
            BridgeEvent::CommitRequested {
                advance: CommitAdvance::Down
            }
        )),
        "Enter must still commit through the editor, got {observed:?}"
    );
    assert!(
        observed
            .iter()
            .any(|e| matches!(e, BridgeEvent::RevertRequested)),
        "Escape must still revert through the editor, got {observed:?}"
    );
    drop(observed);

    // The dtc-lfz.2 carve-out holds here too: Ctrl+Z while the buffer is
    // dirty stays in the textarea (no bubble, default undo untouched).
    type_text(&area, "=A1*3");
    let default_allowed = press_ctrl_key(&area, "z");
    assert_eq!(
        bubbled.lock().unwrap().as_slice(),
        ["s", "o", "F9"],
        "Ctrl+Z on a dirty degrade buffer is text-local and must not reach the shell"
    );
    assert!(
        default_allowed,
        "the local undo carve-out must leave the textarea's native undo intact"
    );
}

/// dtc-j7n8.25 (the regression independent verification found in this bead's
/// first cut): the degrade editor measured "dirty" for its Ctrl+Z/Y carve-out
/// against its mount SEED. The Sheet stage seeds a type-to-replace editor with
/// the typed character — seed == buffer — while the cell's committed text is
/// its authored text (or empty), so that untouched buffer read as CLEAN and
/// Ctrl+Z bubbled to the shell's model Undo under the open editor. With the
/// host's `committed` text supplied, the buffer is dirty from its seed
/// character: Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z stay text-local (stop only, the
/// textarea's native undo intact), Ctrl+S still bubbles, and the F2 mount
/// (seed == committed) keeps the owner-ratified dtc-lfz.2 behavior — an
/// untouched buffer hands Ctrl+Z to the shell, typing makes it local.
#[wasm_bindgen_test]
async fn type_to_replace_seed_is_dirty_against_the_committed_text_so_ctrl_z_stays_local() {
    // The Sheet's type-to-replace mount over a cell whose committed text is `2`.
    let events = Arc::new(Mutex::new(Vec::new()));
    let bubbled = Arc::new(Mutex::new(Vec::new()));
    let host =
        mount_degrade_inside_shell_stub(true, "7", Some("2"), events.clone(), bubbled.clone());
    next_tick().await;
    let area = textarea(&host);
    assert_eq!(
        area.value(),
        "7",
        "the editor seeds with the typed character"
    );

    let default_allowed = press_ctrl_key(&area, "z");
    let seen = bubbled.lock().unwrap().clone();
    assert!(
        seen.is_empty(),
        "Ctrl+Z on an untouched type-to-replace buffer is DIRTY against the committed text and \
         must not reach the shell's model Undo (was: classed clean against the seed and \
         bubbled), got {seen:?}"
    );
    assert!(
        default_allowed,
        "the carve-out never preventDefaults: the textarea's native undo is the effect"
    );
    press_ctrl_key(&area, "y");
    press_ctrl_shift_key(&area, "Z");
    let seen = bubbled.lock().unwrap().clone();
    assert!(
        seen.is_empty(),
        "Ctrl+Y / Ctrl+Shift+Z stay text-local too, got {seen:?}"
    );
    // Dirtiness never touches the shell's own chords: Ctrl+S still bubbles.
    press_ctrl_key(&area, "s");
    assert_eq!(
        bubbled.lock().unwrap().as_slice(),
        ["s"],
        "Ctrl+S bubbles from a dirty type-to-replace buffer"
    );
    let observed = events.lock().unwrap().clone();
    assert!(
        observed.is_empty(),
        "none of the chords commits or reverts the editor, got {observed:?}"
    );

    // Contrast — the F2 / double-click mount: seed == committed, so an untouched
    // buffer is clean and Ctrl+Z bubbles to the shell (dtc-lfz.2, unchanged)...
    let f2_events = Arc::new(Mutex::new(Vec::new()));
    let f2_bubbled = Arc::new(Mutex::new(Vec::new()));
    let f2_host = mount_degrade_inside_shell_stub(
        true,
        "=A2*10",
        Some("=A2*10"),
        f2_events,
        f2_bubbled.clone(),
    );
    next_tick().await;
    let f2_area = textarea(&f2_host);
    press_ctrl_key(&f2_area, "z");
    assert_eq!(
        f2_bubbled.lock().unwrap().as_slice(),
        ["z"],
        "an untouched F2 buffer hands Ctrl+Z to the shell's model Undo"
    );
    // ...and typing on top of it makes it dirty, so Ctrl+Z turns text-local.
    type_text(&f2_area, "=A2*100");
    press_ctrl_key(&f2_area, "z");
    assert_eq!(
        f2_bubbled.lock().unwrap().as_slice(),
        ["z"],
        "Ctrl+Z on the typed-over F2 buffer stays text-local"
    );
}

#[wasm_bindgen_test]
fn ctrl_z_is_consumed_locally_while_dirty_and_bubbles_once_clean() {
    // Bead dtc-lfz.2 / S1.1 (owner-ratified 2026-07-12): the H1b propagation
    // fix above correctly lets un-consumed chords bubble (that's what makes
    // Ctrl+K/F9 work from inside the editor) — but a side effect was that
    // in-textarea Ctrl+Z also bubbled to the shell as workspace Undo,
    // surprising in a text editor. The carve-out: while the local buffer
    // holds uncommitted keystrokes (its text differs from the host's
    // committed `source_text`), Ctrl+Z is consumed locally so the browser's
    // own textarea undo fires on the user's typing instead. Once the buffer
    // is clean again (matches source_text), Ctrl+Z resumes bubbling so the
    // shell's model Undo fires, exactly as before this bead.
    let events = Arc::new(Mutex::new(Vec::new()));
    let bubbled = Arc::new(Mutex::new(Vec::new()));
    let editor = FormulaEditorSurface {
        source_text: "=A1".to_string(),
        document_is_fresh: true,
        ..Default::default()
    };
    let host = mount_inside_shell_stub(
        editor,
        FormulaAssistSurface::default(),
        events.clone(),
        bubbled.clone(),
    );
    let area = textarea(&host);

    // Dirty: the local buffer ("=A1X") no longer matches the committed
    // source_text ("=A1") the surface carries.
    type_text(&area, "=A1X");
    let not_canceled = press_ctrl_key(&area, "z");
    assert!(
        bubbled.lock().unwrap().is_empty(),
        "Ctrl+Z must NOT bubble to the shell while the buffer is dirty, got {:?}",
        bubbled.lock().unwrap()
    );
    assert!(
        not_canceled,
        "the editor must not call preventDefault() when consuming Ctrl+Z — \
         the browser's own textarea undo IS the effect this carve-out \
         exists to produce, so the default action must survive"
    );

    // Clean: type the buffer back to exactly the committed source_text.
    type_text(&area, "=A1");
    press_ctrl_key(&area, "z");
    assert_eq!(
        bubbled.lock().unwrap().as_slice(),
        ["z"],
        "Ctrl+Z must bubble to the shell once the buffer is clean again, \
         so the shell's model Undo verb fires"
    );
}

#[wasm_bindgen_test]
fn ctrl_y_and_ctrl_shift_z_are_also_consumed_locally_while_dirty_but_ctrl_k_is_not() {
    // The carve-out covers the full undo/redo chord family SHELL_SPEC §5.1
    // lists — Ctrl+Z (undo), Ctrl+Y (redo), and Ctrl+Shift+Z (also redo) —
    // not just Ctrl+Z. Ctrl+K is explicitly unaffected by buffer dirtiness
    // (the bead keeps it bubbling regardless), proving the carve-out is
    // scoped to the undo/redo chords only, not "capture everything while
    // dirty".
    let events = Arc::new(Mutex::new(Vec::new()));
    let bubbled = Arc::new(Mutex::new(Vec::new()));
    let editor = FormulaEditorSurface {
        source_text: "=A1".to_string(),
        document_is_fresh: true,
        ..Default::default()
    };
    let host = mount_inside_shell_stub(
        editor,
        FormulaAssistSurface::default(),
        events.clone(),
        bubbled.clone(),
    );
    let area = textarea(&host);

    type_text(&area, "=A1X"); // dirty
    press_ctrl_key(&area, "y");
    press_ctrl_shift_key(&area, "Z");
    assert!(
        bubbled.lock().unwrap().is_empty(),
        "Ctrl+Y and Ctrl+Shift+Z must both be consumed locally while the \
         buffer is dirty, got {:?}",
        bubbled.lock().unwrap()
    );

    // Ctrl+K is unaffected by buffer dirtiness — always bubbles (unchanged
    // from the H1b policy above).
    press_ctrl_key(&area, "k");
    assert_eq!(
        bubbled.lock().unwrap().as_slice(),
        ["k"],
        "Ctrl+K must keep bubbling regardless of buffer dirtiness"
    );
}

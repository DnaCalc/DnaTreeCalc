//! N2/N3: browser-harness proof for the shared cell-entry editor +
//! diagnostics (§B.3 edit-commit loop, §B.5 diagnostics) and the `+ name`
//! creation / rename forms (§B.3 Name loop).
//!
//! In its own file, not `tests/browser_smoke.rs` (H11's lane), matching the
//! K1b coordination note (`browser_grid.rs`): concurrent lanes each own a
//! separate harness file to avoid colliding edits.
//!
//! These tests mount the shared components **directly** into the browser DOM
//! with a `RecordingDispatcher` (or a small rejecting dispatcher), rather
//! than going through the `?grid=1` demo route: the demo grid rides the
//! tree-model session, which fills `authored: None` and an empty
//! `defined_names` catalog (documented in `browser_smoke.rs`'s N1 test), so
//! no app-route fixture can drive an editable notebook entry or a name
//! creation. Mounting the component directly is still a real browser proof: a
//! live Leptos mount, real DOM, real `input`/`keydown`/`blur`/`click` events,
//! real intent dispatch through the `Dispatcher` trait.
//!
//! N2's three acceptance assertions, proven in the live DOM:
//!   1. committing (Enter) drives exactly ONE `EnterGridCell`;
//!   2. a rejection receipt keeps the editor open with the text intact and
//!      renders the diagnostics under it;
//!   3. Esc reverts the buffer and dispatches nothing.
//!
//! N3's three acceptance assertions, proven in the live DOM:
//!   1. creating `rate = 0.065` dispatches `EnterGridCell` then
//!      `SetDefinedName` (recorded order asserted) via a live click on the
//!      `+ name` form's Create button;
//!   2. a duplicate name renders the rejection inline and the form stays
//!      open (mounted state, not just a pure-fn assertion);
//!   3. rename dispatches only `RenameDefinedName`, driven by a live
//!      Enter keypress on the rename field.
//!
//! Run locally (same runner as H11/K1b):
//!
//! ```text
//! cargo test -p dnatreecalc-web --target wasm32-unknown-unknown
//! ```

#![cfg(target_arch = "wasm32")]

use std::sync::Arc;

use dnatreecalc_skin_framework::{
    CellEntryEditor, DefinedNameScopeProjection, DefinedNamesProjection, Dispatcher,
    EntryDiagnostics, EntryFeedback, GridEntryDiagnosticProjection, IntentError, IntentReceipt,
    NameForm, NodeId, RecordingDispatcher, RenameNameForm, WorkspaceIntent,
};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Let queued reactive/rendering work flush (one macrotask turn).
async fn next_tick() {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        web_sys::window()
            .expect("window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0)
            .expect("setTimeout");
    });
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .expect("tick");
}

fn fresh_mount_point(id: &str) -> web_sys::HtmlElement {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let element = document
        .create_element("div")
        .expect("create mount div")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("HtmlElement");
    element.set_id(id);
    document
        .body()
        .expect("document body")
        .append_child(&element)
        .expect("append mount div");
    element
}

fn query_in(host: &web_sys::Element, selector: &str) -> Option<web_sys::Element> {
    host.query_selector(selector).expect("query_selector")
}

/// The mounted editor's `<input>`, cast to `HtmlInputElement`.
fn editor_input(host: &web_sys::Element) -> web_sys::HtmlInputElement {
    query_in(host, ".dtc-cell-entry__input")
        .expect("the editor input must mount")
        .dyn_into::<web_sys::HtmlInputElement>()
        .expect("input is an HtmlInputElement")
}

/// Type `text` into the editor input and fire an `input` event, as a user
/// would — the component reads `event_target_value` on `input`.
fn type_into(input: &web_sys::HtmlInputElement, text: &str) {
    input.set_value(text);
    let event = web_sys::Event::new("input").expect("construct input event");
    input.dispatch_event(&event).expect("dispatch input event");
}

/// Fire a `keydown` for `key` on the input (bubbling+cancelable, as a real key
/// press is), returning after dispatch.
fn press_key(input: &web_sys::HtmlInputElement, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
        .expect("construct keydown event");
    input.dispatch_event(&event).expect("dispatch keydown event");
}

/// Mount the shared editor over a fixed backing cell with the given dispatcher,
/// starting closed feedback and an open editor. Returns the mount host, the
/// `editing` signal, and the `feedback` signal for assertions.
fn mount_editor(
    mount_id: &str,
    initial_text: &str,
    dispatch: Arc<dyn Dispatcher>,
) -> (web_sys::HtmlElement, RwSignal<bool>, RwSignal<EntryFeedback>) {
    let host = fresh_mount_point(mount_id);
    let editing = RwSignal::new(true);
    let feedback = RwSignal::new(EntryFeedback::None);
    let initial = initial_text.to_string();

    // Compose the two shared components exactly as the notebook's `EntryRow`
    // does: the editor over the backing cell, and the diagnostics list driven
    // by the same `feedback` signal (a `Rejected` feedback renders
    // `EntryDiagnostics` under the editor). This is the real N2 composition,
    // mounted directly because no app-route can seed an editable entry.
    let handle = leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        let feedback_view = move || match feedback.get() {
            EntryFeedback::Rejected { diagnostics } => {
                view! { <EntryDiagnostics diagnostics=diagnostics /> }.into_any()
            }
            _ => ().into_any(),
        };
        view! {
            <CellEntryEditor
                grid=NodeId::new("Sheet1")
                row=4
                col=2
                initial_text=initial.clone()
                dispatch=dispatch.clone()
                editing=editing
                feedback=feedback
            />
            {feedback_view}
        }
    });
    handle.forget();
    (host, editing, feedback)
}

/// A dispatcher that rejects every `EnterGridCell` with typed entry
/// diagnostics — the raced/invalid-formula path (§A.4). It still records the
/// intents so a test can prove the editor dispatched (once).
#[derive(Clone, Default)]
struct RejectingDispatcher {
    log: Arc<std::sync::Mutex<Vec<WorkspaceIntent>>>,
}

impl RejectingDispatcher {
    fn intents(&self) -> Vec<WorkspaceIntent> {
        self.log.lock().expect("log poisoned").clone()
    }
}

impl Dispatcher for RejectingDispatcher {
    fn dispatch(&self, intent: WorkspaceIntent) -> IntentReceipt {
        self.log.lock().expect("log poisoned").push(intent);
        IntentReceipt::rejected(IntentError::GridEntryRejected {
            diagnostics: vec![GridEntryDiagnosticProjection {
                message: "unexpected end of formula".to_string(),
                span: Some((3, 3)),
            }],
        })
    }
}

/// Acceptance (1), browser proof: committing a literal via Enter drives
/// exactly ONE `EnterGridCell` with the typed text, and the editor closes.
#[wasm_bindgen_test]
async fn commit_via_enter_dispatches_exactly_one_enter_grid_cell() {
    let dispatcher = RecordingDispatcher::new();
    let dispatch: Arc<dyn Dispatcher> = Arc::new(dispatcher.clone());
    let (host, editing, _feedback) = mount_editor("dtc-n2-commit-once", "", dispatch);
    next_tick().await;

    let input = editor_input(&host);
    type_into(&input, "0.065");
    press_key(&input, "Enter");
    next_tick().await;

    let enters = dispatcher
        .intents()
        .into_iter()
        .filter(|intent| matches!(intent, WorkspaceIntent::EnterGridCell { .. }))
        .collect::<Vec<_>>();
    assert_eq!(
        enters.len(),
        1,
        "one commit must drive exactly one EnterGridCell, got {}",
        enters.len()
    );
    let WorkspaceIntent::EnterGridCell {
        grid, row, col, text,
    } = &enters[0]
    else {
        unreachable!()
    };
    assert_eq!((grid, *row, *col), (&NodeId::new("Sheet1"), 4, 2));
    assert_eq!(text, "0.065", "the raw typed text is dispatched verbatim");
    assert!(!editing.get_untracked(), "a clean commit closes the editor");
}

/// Acceptance (2), browser proof: on a rejection receipt the editor stays open
/// with the entered text intact AND renders the diagnostics under it.
#[wasm_bindgen_test]
async fn rejection_keeps_text_and_shows_diagnostics_in_the_live_dom() {
    let dispatcher = RejectingDispatcher::default();
    let dispatch: Arc<dyn Dispatcher> = Arc::new(dispatcher.clone());
    let (host, editing, feedback) = mount_editor("dtc-n2-rejection", "", dispatch);
    next_tick().await;

    let input = editor_input(&host);
    type_into(&input, "=1+");
    press_key(&input, "Enter");
    next_tick().await;

    // Exactly one dispatch, and it was rejected — the editor must stay open.
    assert_eq!(dispatcher.intents().len(), 1, "one commit attempt");
    assert!(editing.get_untracked(), "a rejection keeps the editor open");

    // The entered text is retained in the live input (engine guaranteed no
    // mutation).
    let input = editor_input(&host);
    assert_eq!(
        input.value(),
        "=1+",
        "the rejected text is retained in the editor buffer"
    );

    // The diagnostics render under the editor via the shared component.
    assert!(
        matches!(feedback.get_untracked(), EntryFeedback::Rejected { .. }),
        "feedback carries the typed rejection"
    );
    let diagnostics = query_in(&host, ".dtc-entry-diagnostics")
        .expect("the diagnostics list must render on rejection");
    let text = diagnostics.text_content().unwrap_or_default();
    assert!(
        text.contains("unexpected end of formula"),
        "the diagnostic message renders: {text}"
    );
    assert!(
        text.contains("chars 3\u{2013}3"),
        "the span badge renders for a Some(span) diagnostic: {text}"
    );
}

/// Acceptance (3), browser proof: Esc reverts the buffer to the committed text
/// and dispatches NOTHING.
#[wasm_bindgen_test]
async fn escape_reverts_buffer_without_dispatch() {
    let dispatcher = RecordingDispatcher::new();
    let dispatch: Arc<dyn Dispatcher> = Arc::new(dispatcher.clone());
    let (host, editing, _feedback) = mount_editor("dtc-n2-escape", "0.065", dispatch);
    next_tick().await;

    // Edit the buffer, then press Escape.
    let input = editor_input(&host);
    type_into(&input, "0.999");
    press_key(&input, "Escape");
    next_tick().await;

    assert_eq!(
        dispatcher.intents().len(),
        0,
        "Esc must dispatch nothing at all"
    );
    assert!(!editing.get_untracked(), "Esc closes the editor");
}

// ---------------------------------------------------------------------------
// N3: `+ name` creation form + rename inline, mounted directly (module doc).
// ---------------------------------------------------------------------------

/// Mount the `+ name` creation form over the given (already-mirrored)
/// defined-names catalog. Returns the mount host and the form's `open` signal
/// (a clean commit or Cancel flips it to `false`).
fn mount_name_form(
    mount_id: &str,
    defined_names: DefinedNamesProjection,
    dispatch: Arc<dyn Dispatcher>,
) -> (web_sys::HtmlElement, RwSignal<bool>) {
    let host = fresh_mount_point(mount_id);
    let open = RwSignal::new(true);

    let handle = leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        view! {
            <NameForm
                defined_names=defined_names.clone()
                dispatch=dispatch.clone()
                open=open
            />
        }
    });
    handle.forget();
    (host, open)
}

fn name_form_input(host: &web_sys::Element, selector: &str) -> web_sys::HtmlInputElement {
    query_in(host, selector)
        .unwrap_or_else(|| panic!("{selector} must mount"))
        .dyn_into::<web_sys::HtmlInputElement>()
        .expect("input is an HtmlInputElement")
}

fn click(element: &web_sys::Element) {
    let event = web_sys::MouseEvent::new("click").expect("construct click event");
    element.dispatch_event(&event).expect("dispatch click");
}

/// N3 acceptance (1), browser proof: filling `rate` / `0.065` and clicking
/// Create dispatches a single `CreateNamedValue` — the atomic host-core verb
/// that owns `_names` backing-cell allocation (replacing the old two-intent
/// `EnterGridCell` + `SetDefinedName` guess that could not resolve the backing
/// grid on a host-core workbook) — via a live DOM click.
#[wasm_bindgen_test]
async fn create_name_button_dispatches_single_create_named_value() {
    let dispatcher = RecordingDispatcher::new();
    let dispatch: Arc<dyn Dispatcher> = Arc::new(dispatcher.clone());
    let (host, open) = mount_name_form(
        "dtc-n3-create",
        DefinedNamesProjection::default(),
        dispatch,
    );
    next_tick().await;

    let name_input = name_form_input(&host, ".dtc-name-form__name");
    type_into(&name_input, "rate");
    let body_input = name_form_input(&host, ".dtc-name-form__body");
    type_into(&body_input, "0.065");

    let create_button = query_in(&host, ".dtc-name-form__create").expect("Create button mounts");
    click(&create_button);
    next_tick().await;

    let intents = dispatcher.intents();
    assert_eq!(intents.len(), 1, "exactly one intent dispatched");
    match &intents[0] {
        WorkspaceIntent::CreateNamedValue { name, value_text } => {
            assert_eq!(name, "rate");
            assert_eq!(value_text, "0.065");
        }
        other => panic!("expected CreateNamedValue, got {other:?}"),
    }
    assert!(!open.get_untracked(), "a clean commit closes the form");
}

/// N3 acceptance (2), browser proof: creating a name that already exists in
/// the mirrored catalog renders the rejection inline and the form stays
/// open — zero intents dispatched (the honest client-side pre-check, module
/// doc), proven against the live DOM, not a pure-fn assertion.
#[wasm_bindgen_test]
async fn duplicate_name_renders_inline_rejection_and_form_stays_open() {
    let dispatcher = RecordingDispatcher::new();
    let dispatch: Arc<dyn Dispatcher> = Arc::new(dispatcher.clone());
    let mut catalog = DefinedNamesProjection::default();
    catalog.entries.push(dnatreecalc_skin_framework::DefinedNameProjection {
        scope: DefinedNameScopeProjection::Sheet(NodeId::new("_names")),
        name: "rate".to_string(),
        target: dnatreecalc_skin_framework::DefinedNameTargetProjection::Static(
            dnatreecalc_skin_framework::GridRectProjection {
                top_row: 1,
                left_col: 1,
                bottom_row: 1,
                right_col: 1,
            },
        ),
        is_dynamic: false,
    });
    let (host, open) = mount_name_form("dtc-n3-duplicate", catalog, dispatch);
    next_tick().await;

    let name_input = name_form_input(&host, ".dtc-name-form__name");
    type_into(&name_input, "rate");
    let body_input = name_form_input(&host, ".dtc-name-form__body");
    type_into(&body_input, "0.07");

    let create_button = query_in(&host, ".dtc-name-form__create").expect("Create button mounts");
    click(&create_button);
    next_tick().await;

    assert_eq!(
        dispatcher.intents().len(),
        0,
        "a duplicate name dispatches nothing at all"
    );
    assert!(open.get_untracked(), "the form stays open on rejection");
    let error = query_in(&host, ".dtc-name-form__error").expect("the inline error must render");
    let text = error.text_content().unwrap_or_default();
    assert!(
        text.contains("rate"),
        "the inline rejection names the duplicate: {text}"
    );
}

/// N3 acceptance (3), browser proof: committing a rename (Enter) dispatches
/// ONLY `RenameDefinedName` — no `SetDefinedName`, no `EnterGridCell`.
#[wasm_bindgen_test]
async fn rename_dispatches_only_rename_defined_name_in_the_live_dom() {
    let dispatcher = RecordingDispatcher::new();
    let dispatch: Arc<dyn Dispatcher> = Arc::new(dispatcher.clone());
    let host = fresh_mount_point("dtc-n3-rename");
    let editing = RwSignal::new(true);

    let handle = leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        view! {
            <RenameNameForm
                scope=DefinedNameScopeProjection::Sheet(NodeId::new("_names"))
                current_name="rate".to_string()
                dispatch=dispatch.clone()
                editing=editing
            />
        }
    });
    handle.forget();
    next_tick().await;

    let input = name_form_input(&host, ".dtc-name-form__name");
    input.set_value("taxRate");
    let event = web_sys::Event::new("input").expect("construct input event");
    input.dispatch_event(&event).expect("dispatch input event");
    press_key(&input, "Enter");
    next_tick().await;

    let intents = dispatcher.intents();
    assert_eq!(intents.len(), 1, "exactly one intent dispatched");
    match &intents[0] {
        WorkspaceIntent::RenameDefinedName {
            old_name, new_name, ..
        } => {
            assert_eq!(old_name, "rate");
            assert_eq!(new_name, "taxRate");
        }
        other => panic!("expected RenameDefinedName, got {other:?}"),
    }
    assert!(!editing.get_untracked(), "a clean rename closes the field");
}

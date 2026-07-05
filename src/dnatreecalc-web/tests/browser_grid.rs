//! K1b: browser-harness proof for the grid canvas upgrades (interest
//! coalescing + authored-aware cell render + show-formulas mode).
//!
//! In its own file, not `tests/browser_smoke.rs` (H11's lane), per the K1b
//! coordination note: K1b and N1 run concurrently and both extend the H11
//! harness, so each owns a separate test file to avoid colliding edits to
//! the same file.
//!
//! Most of these tests mount the shared `grid_canvas::grid_surface` component
//! **directly** into the browser DOM with a synthetic `WorkspaceState`
//! carrying real authored metadata, rather than going through the `?grid=1`
//! demo route: at K1b/N1 authoring time the demo grid rode the tree-model
//! session, whose grid projection hardcoded `authored: None` (H3 scoped the
//! authored-view fill to the workbook host-core path only), so the app-route
//! fixture could not exercise K1b's authored-aware branches. Mounting the
//! component directly was still a real browser proof: a live Leptos mount,
//! real DOM, real scroll events, real `requestAnimationFrame` coalescing.
//!
//! dtc-ajl.30 closed that gap: `TreeWorkspaceSession`'s grid projection
//! (`dnatreecalc-host/src/app/session.rs`, `grid_projection_for`) now fills
//! `authored` from the engine's `grid_authored_view`, mirroring host-core's
//! workbook fill (`dnacalc-host-core/src/grid_publication.rs`). The
//! `grid_1_route_renders_authored_formula_text_via_show_formulas_toggle` test
//! below is the route-level proof this file's tests couldn't offer before:
//! it drives the real `?grid=1` route end-to-end rather than mounting the
//! component with a fixture.
//!
//! Run locally (same runner as H11):
//!
//! ```text
//! cargo test -p dnatreecalc-web --target wasm32-unknown-unknown
//! ```

#![cfg(target_arch = "wasm32")]

use std::sync::Arc;

use dnatreecalc_skin_framework::{
    Dispatcher, GridAuthoredCellProjection, GridAuthoredKindProjection, GridCellProjection,
    GridCellRefProjection, GridEditabilityProjection, GridOverlayBundle, GridProjection, NodeId,
    NodeKey, NodeValueProjection, RecordingDispatcher, WorkspaceIntent, WorkspaceState,
};
use dnatreecalc_skins::grid_canvas::{GRID_CANVAS_CSS, grid_surface};
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

/// Wait one animation frame (the interest-coalescer's flush boundary).
async fn next_animation_frame() {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let _ = web_sys::window()
            .expect("window")
            .request_animation_frame(&resolve);
    });
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .expect("raf");
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

/// A synthetic grid projection with real authored metadata: B1 is a formula
/// cell (`=A1*3`, value 30, Editable), B4 is a `SpillDisplay` member
/// anchored at B3, and A1 is a literal 10.
fn authored_grid_state(grid_id: &NodeId) -> WorkspaceState {
    let cells = vec![
        GridCellProjection {
            row: 1,
            col: 1,
            value: NodeValueProjection::Number {
                raw: "10".to_string(),
                display: "10".to_string(),
            },
            value_epoch: 1,
            authored: Some(GridAuthoredCellProjection {
                row: 1,
                col: 1,
                kind: GridAuthoredKindProjection::Literal,
                literal_text: Some("10".to_string()),
                source_text: None,
                editability: GridEditabilityProjection::Editable,
            }),
            provenance: None,
        },
        GridCellProjection {
            row: 1,
            col: 2,
            value: NodeValueProjection::Number {
                raw: "30".to_string(),
                display: "30".to_string(),
            },
            value_epoch: 1,
            authored: Some(GridAuthoredCellProjection {
                row: 1,
                col: 2,
                kind: GridAuthoredKindProjection::Formula,
                literal_text: None,
                source_text: Some("=A1*3".to_string()),
                editability: GridEditabilityProjection::Editable,
            }),
            provenance: None,
        },
        GridCellProjection {
            row: 4,
            col: 2,
            value: NodeValueProjection::Number {
                raw: "70".to_string(),
                display: "70".to_string(),
            },
            value_epoch: 1,
            authored: Some(GridAuthoredCellProjection {
                row: 4,
                col: 2,
                kind: GridAuthoredKindProjection::Empty,
                literal_text: None,
                source_text: None,
                editability: GridEditabilityProjection::SpillDisplay {
                    anchor: GridCellRefProjection { row: 3, col: 2 },
                },
            }),
            provenance: None,
        },
    ];
    let grid = GridProjection {
        grid_node_key: NodeKey::new("sheet"),
        grid_node_id: grid_id.clone(),
        grid_id: "book:grid:sheet:grid".to_string(),
        max_rows: 100,
        max_cols: 10,
        cells,
        projection_epoch: 1,
        overlays: GridOverlayBundle::default(),
        overlay_epoch: 0,
        differential_clean: true,
        authored_epoch: 1,
    };
    let mut state = WorkspaceState::default();
    state.grids.insert(grid_id.clone(), grid);
    state
}

/// Mount the shared grid surface directly with the authored fixture; returns
/// the mount host, the show-formulas toggle signal, and the recording
/// dispatcher for intent assertions.
fn mount_grid_fixture(
    mount_id: &str,
) -> (web_sys::HtmlElement, RwSignal<bool>, RecordingDispatcher) {
    let host = fresh_mount_point(mount_id);
    let grid_id = NodeId::new("Sheet1");
    let state = authored_grid_state(&grid_id);
    let show_formulas = RwSignal::new(false);
    let dispatcher = RecordingDispatcher::new();
    let dispatch: Arc<dyn Dispatcher> = Arc::new(dispatcher.clone());

    let mount_grid_id = grid_id.clone();
    let handle = leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        let (workspace, _set_workspace) = signal(state);
        view! {
            <style>{GRID_CANVAS_CSS}</style>
            {grid_surface(
                mount_grid_id,
                workspace,
                dispatch,
                show_formulas.into(),
            )}
        }
    });
    handle.forget();
    (host, show_formulas, dispatcher)
}

/// Mount the shared grid surface with a caller-supplied dispatcher (K2's
/// reject/Esc reruns need a dispatcher other than `RecordingDispatcher`'s
/// always-accept behavior). Returns the mount host.
fn mount_grid_fixture_with_dispatch(
    mount_id: &str,
    dispatch: Arc<dyn Dispatcher>,
) -> web_sys::HtmlElement {
    let host = fresh_mount_point(mount_id);
    let grid_id = NodeId::new("Sheet1");
    let state = authored_grid_state(&grid_id);
    let show_formulas = RwSignal::new(false);

    let mount_grid_id = grid_id.clone();
    let handle = leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        let (workspace, _set_workspace) = signal(state);
        view! {
            <style>{GRID_CANVAS_CSS}</style>
            {grid_surface(
                mount_grid_id,
                workspace,
                dispatch,
                show_formulas.into(),
            )}
        }
    });
    handle.forget();
    host
}

/// Acceptance (3), browser proof: show-formulas mode renders
/// `authored.source_text` in place of the computed value — flipping the mode
/// signal swaps the formula cell's rendered text from `30` to `=A1*3` and
/// back, while the literal cell keeps rendering its value throughout.
#[wasm_bindgen_test]
async fn show_formulas_mode_swaps_formula_cell_text_in_the_live_dom() {
    let (host, show_formulas, _dispatcher) = mount_grid_fixture("dtc-k1b-show-formulas");
    next_tick().await;

    let grid_text = || {
        query_in(&host, ".dtc-grid")
            .expect("grid surface must mount")
            .text_content()
            .unwrap_or_default()
    };

    let before = grid_text();
    assert!(
        before.contains("30") && !before.contains("=A1*3"),
        "mode off: the computed value renders, not source text: {before}"
    );

    show_formulas.set(true);
    next_tick().await;

    let during = grid_text();
    assert!(
        during.contains("=A1*3"),
        "mode on: the formula cell renders its authored source text: {during}"
    );
    assert!(
        during.contains("10"),
        "mode on: the literal cell still renders its value: {during}"
    );

    show_formulas.set(false);
    next_tick().await;

    let after = grid_text();
    assert!(
        after.contains("30") && !after.contains("=A1*3"),
        "mode off again: back to computed values: {after}"
    );
}

/// Acceptance (2), browser proof: a `SpillDisplay` cell renders the
/// read-only affordance (class + anchor hint) from the projection in the
/// live DOM, while `Editable` cells render unmarked.
#[wasm_bindgen_test]
async fn spill_display_cell_carries_read_only_affordance_in_the_live_dom() {
    let (host, _show_formulas, _dispatcher) = mount_grid_fixture("dtc-k1b-spill-affordance");
    next_tick().await;

    let spill = query_in(&host, ".dtc-grid__cell--spill-display")
        .expect("the SpillDisplay cell must render its affordance class");
    assert!(
        spill
            .get_attribute("title")
            .unwrap_or_default()
            .contains("Spilled from B3"),
        "the SpillDisplay cell carries the anchor hint"
    );
    assert!(
        spill.text_content().unwrap_or_default().contains("70"),
        "the SpillDisplay cell still shows its spilled value"
    );

    // Exactly one cell carries an *editability* affordance modifier — the two
    // Editable cells render unmarked. `dtc-grid__cell--selected` (K2, §C.3)
    // is a distinct selection concept, not an editability affordance, so it
    // is excluded here: K2 defaults the anchor to (1, 1) on mount, which
    // would otherwise also match this selector and make this pre-K2
    // assertion about editability affordances collide with K2's unrelated
    // selection-outline class.
    let marked = host
        .query_selector_all(
            ".dtc-grid__cell--spill-display, \
             .dtc-grid__cell--repeated-member, \
             .dtc-grid__cell--merged-follower, \
             .dtc-grid__cell--table-structural",
        )
        .expect("query affordance cells");
    assert_eq!(
        marked.length(),
        1,
        "only the SpillDisplay cell carries an editability affordance modifier"
    );
}

/// Point the page URL's query string at `search` without reloading,
/// preserving path + hash. Mirrors `browser_smoke.rs`'s helper of the same
/// shape (duplicated rather than shared across files, matching the K1b/N1
/// per-file test-harness split already in place for this crate).
fn set_url_search(search: &str) {
    let window = web_sys::window().expect("window");
    let location = window.location();
    let url = format!(
        "{}{}{}",
        location.pathname().expect("pathname"),
        search,
        location.hash().expect("hash"),
    );
    window
        .history()
        .expect("history")
        .replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url))
        .expect("replaceState");
}

/// Find and click a skin-switcher tab (or any button) by its visible text.
fn find_by_text(host: &web_sys::Element, selector: &str, text: &str) -> Option<web_sys::HtmlElement> {
    let nodes = host.query_selector_all(selector).expect("query_selector_all");
    for index in 0..nodes.length() {
        let node = nodes.item(index)?;
        let element: web_sys::HtmlElement = node.dyn_into().expect("element is an HtmlElement");
        if element.text_content().unwrap_or_default().contains(text) {
            return Some(element);
        }
    }
    None
}

/// dtc-ajl.30 route-level proof: the `?grid=1` demo route now carries
/// authored metadata all the way through the real `mount_dnatreecalc` app —
/// not just the direct-fixture mount the rest of this file uses. Before this
/// bead, `TreeWorkspaceSession`'s grid projection hardcoded
/// `GridCellProjection::authored: None` (`dnatreecalc-host/src/app/session.rs`,
/// `grid_projection_for`), so the Sheet lens's "Show formulas" toggle had
/// nothing to render for the demo grid's column-B formula cells (`=A1*10`,
/// seeded by `TreeWorkspaceSession::attach_demo_grid`) — this test would have
/// failed on old `main` because the toggle button flips a mode with no
/// authored data behind it, so `show_formulas_mode` would render blank/no
/// `=A1*10` text anywhere in the grid.
///
/// Drives the real production path end-to-end: URL `?grid=1` ->
/// `mount_dnatreecalc` -> `attach_demo_grid` -> `TreeWorkspaceSession`'s grid
/// projection (now filled from the engine's `grid_authored_view`) -> the
/// Sheet lens's grid surface -> the "Show formulas" toggle -> the DOM.
#[wasm_bindgen_test]
async fn grid_1_route_renders_authored_formula_text_via_show_formulas_toggle() {
    set_url_search("?grid=1");
    let host = fresh_mount_point("dtc-ajl30-grid-route-authored");

    dnatreecalc_web::mount_dnatreecalc("dtc-ajl30-grid-route-authored")
        .expect("mount_dnatreecalc must succeed");
    next_tick().await;

    // Switch to the Sheet lens, same as the H11 demo-grid smoke test.
    let tabs = host
        .query_selector_all(".dtc-skin-switcher__tab")
        .expect("query tabs");
    let mut sheet_tab = None;
    for index in 0..tabs.length() {
        let Some(node) = tabs.item(index) else {
            continue;
        };
        let element: web_sys::HtmlElement = node.dyn_into().expect("tab is an HtmlElement");
        if element.text_content().unwrap_or_default().contains("Sheet") {
            sheet_tab = Some(element);
            break;
        }
    }
    sheet_tab
        .expect("the shell must render a Sheet lens tab")
        .click();
    next_tick().await;

    assert!(
        query_in(&host, ".dtc-grid").is_some(),
        "?grid=1 must render the demo grid surface in the Sheet lens"
    );

    // Before toggling, the computed values render (e.g. column B's `=A1*10`
    // shows its number, not its source text).
    let grid_text = || {
        query_in(&host, ".dtc-grid")
            .expect("grid surface must mount")
            .text_content()
            .unwrap_or_default()
    };
    let before = grid_text();
    assert!(
        !before.contains("=A1*10"),
        "before toggling show-formulas, the demo grid renders computed values, not source text: {before}"
    );

    // Flip the real "Show formulas" toolbar button in the live DOM.
    find_by_text(&host, "button", "Show formulas")
        .expect("the Sheet lens must render a Show formulas toggle button")
        .click();
    next_tick().await;

    // The route's authored fill (dtc-ajl.30) must now surface the demo
    // grid's authored formula source text through the real projection path —
    // this is the assertion that was structurally impossible before this
    // bead, per this file's header note.
    let during = grid_text();
    assert!(
        during.contains("=A1*10"),
        "show-formulas mode over the ?grid=1 route must render the demo grid's \
         authored formula source text (proves TreeWorkspaceSession's grid \
         projection now fills `authored`, not just the direct-fixture mount): {during}"
    );
}

/// Acceptance (1), browser proof with a dispatch-count assertion: a storm of
/// N scroll events fired within one frame produces exactly ONE
/// `SetGridInterest` dispatch after the next animation frame (the
/// coalescer's flush boundary) — not N.
#[wasm_bindgen_test]
async fn scroll_storm_within_one_frame_dispatches_exactly_one_interest() {
    let (host, _show_formulas, dispatcher) = mount_grid_fixture("dtc-k1b-scroll-storm");
    next_tick().await;

    let grid = query_in(&host, ".dtc-grid").expect("grid surface must mount");
    let grid: web_sys::HtmlElement = grid.dyn_into().expect("grid is an HtmlElement");

    assert_eq!(
        dispatcher.intents().len(),
        0,
        "no interest dispatch before any scroll"
    );

    // N scroll events within the same task — all before the next animation
    // frame can fire.
    for step in 0..20 {
        grid.set_scroll_top(step * 22);
        let event = web_sys::Event::new("scroll").expect("construct scroll event");
        grid.dispatch_event(&event).expect("dispatch scroll event");
    }

    // Let the coalescer's RAF callback run, plus one macrotask for safety.
    next_animation_frame().await;
    next_tick().await;

    let intents = dispatcher.intents();
    let interest_count = intents
        .iter()
        .filter(|intent| matches!(intent, WorkspaceIntent::SetGridInterest { .. }))
        .count();
    assert_eq!(
        interest_count, 1,
        "N scroll events in one frame must coalesce to exactly one SetGridInterest, got {interest_count}"
    );
}

// ---------------------------------------------------------------------------
// K2: in-grid cell edit loop (§C.3) browser proofs.
// ---------------------------------------------------------------------------

/// The mounted editor's `<input>`, cast to `HtmlInputElement` (K2's editor is
/// the same shared `CellEntryEditor` N2/N3 mount, so this mirrors
/// `browser_notebook.rs`'s helper of the same shape — duplicated rather than
/// shared across files, per this crate's per-file harness split).
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

/// Fire a `keydown` for `key` on the input (bubbling+cancelable, as a real
/// key press is).
fn press_key(input: &web_sys::HtmlInputElement, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
        .expect("construct keydown event");
    input
        .dispatch_event(&event)
        .expect("dispatch keydown event");
}

/// Fire a `keydown` for `key` on any event target (the grid container's own
/// keyboard model, §C.3, fires while the selected cell is not being edited —
/// unlike `press_key`, which targets the mounted editor's `<input>`).
fn press_key_on(target: &web_sys::EventTarget, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
        .expect("construct keydown event");
    target
        .dispatch_event(&event)
        .expect("dispatch keydown event");
}

/// K2 acceptance, browser proof: click a cell to select+edit it, type text,
/// press Enter — exactly one `EnterGridCell` is dispatched and the editor
/// closes back to the read-only value display (the "value renders" half of
/// the loop: the live DOM stops showing an `<input>` and goes back to a
/// plain value cell, proving the commit-and-close path actually ran, not
/// just that the intent was queued).
#[wasm_bindgen_test]
async fn click_type_enter_commits_exactly_one_entergridcell_and_value_renders() {
    let (host, _show_formulas, dispatcher) = mount_grid_fixture("dtc-k2-click-type-enter");
    next_tick().await;

    // A1 (row 1, col 1) is a plain Editable literal in the fixture.
    let cells = host
        .query_selector_all(".dtc-grid__cell")
        .expect("query cells");
    let a1: web_sys::HtmlElement = cells
        .item(0)
        .expect("A1 must render")
        .dyn_into()
        .expect("cell is an HtmlElement");
    assert!(
        query_in(&host, ".dtc-cell-entry__input").is_none(),
        "no editor mounted before any click"
    );

    a1.click();
    next_tick().await;

    let input = editor_input(&host);
    type_into(&input, "99");
    press_key(&input, "Enter");
    next_tick().await;

    let intents = dispatcher.intents();
    let entered: Vec<_> = intents
        .iter()
        .filter(|intent| matches!(intent, WorkspaceIntent::EnterGridCell { .. }))
        .collect();
    assert_eq!(
        entered.len(),
        1,
        "typing and pressing Enter must dispatch exactly one EnterGridCell, got {}",
        entered.len()
    );
    let WorkspaceIntent::EnterGridCell { row, col, text, .. } = entered[0] else {
        unreachable!()
    };
    assert_eq!((*row, *col), (1, 1), "the commit targets the clicked cell");
    assert_eq!(text, "99", "the commit carries the typed text");

    // The editor closed (a bare `RecordingDispatcher::accepted()` receipt is
    // a clean commit, §B.3 step 3) — the cell goes back to its read-only
    // value display, proving the loop actually completed rather than
    // leaving the editor stuck open.
    assert!(
        query_in(&host, ".dtc-cell-entry__input").is_none(),
        "a clean commit closes the editor back to the value display"
    );
    let cell_text = a1.text_content().unwrap_or_default();
    assert!(
        !cell_text.is_empty(),
        "the cell renders a value after the editor closes: {cell_text:?}"
    );
}

/// K2 acceptance (3), browser proof: an edit attempt (click) on a
/// `SpillDisplay` cell dispatches nothing and focuses (jumps the selection
/// to) the anchor cell, surfacing the anchor hint — the editor never mounts
/// over the read-only spill cell.
#[wasm_bindgen_test]
async fn click_on_spill_display_cell_dispatches_nothing_and_jumps_to_anchor() {
    let (host, _show_formulas, dispatcher) = mount_grid_fixture("dtc-k2-spill-rejection");
    next_tick().await;

    let spill: web_sys::HtmlElement = query_in(&host, ".dtc-grid__cell--spill-display")
        .expect("the SpillDisplay cell (B4) must render its affordance class")
        .dyn_into()
        .expect("cell is an HtmlElement");

    assert_eq!(dispatcher.intents().len(), 0, "no intents before the click");

    spill.click();
    next_tick().await;

    // Still nothing dispatched: a non-Editable cell's edit attempt never
    // reaches the engine (§C.3 acceptance (3)).
    assert_eq!(
        dispatcher.intents().len(),
        0,
        "an edit attempt on a SpillDisplay cell must dispatch nothing"
    );
    // No editor mounted over the spill cell.
    assert!(
        query_in(&host, ".dtc-cell-entry__input").is_none(),
        "a SpillDisplay cell never opens the editor"
    );
    // The status hint names the anchor — "focuses the anchor" surfaced, not
    // a silent no-op.
    let hint = query_in(&host, ".dtc-grid__status-hint")
        .expect("a status hint must appear after the jump attempt")
        .text_content()
        .unwrap_or_default();
    assert!(
        hint.contains("Spilled from B3"),
        "the status hint names the anchor cell: {hint:?}"
    );
    // B3 (the anchor) is outside this fixture's rendered cells (only A1, B1,
    // B4 exist), so no cell in the DOM carries the selected class — proving
    // the jump moved selection away from the spill cell rather than leaving
    // it selected there.
    let selected = host
        .query_selector_all(".dtc-grid__cell--selected")
        .expect("query selected cells");
    assert_eq!(
        selected.length(),
        0,
        "the anchor cell (B3) is outside the rendered fixture, so nothing \
         in the DOM still carries the selected class after the jump"
    );
}

/// A dispatcher that always rejects with typed `GridEntryRejected`
/// diagnostics — the raced/invalid-formula path (§A.4). It still records the
/// intents so a test can prove the editor dispatched (once). Mirrors
/// `browser_notebook.rs`'s `RejectingDispatcher` (N2's own acceptance (2)
/// double), duplicated per this crate's per-file harness split.
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
    fn dispatch(&self, intent: WorkspaceIntent) -> dnatreecalc_skin_framework::IntentReceipt {
        self.log.lock().expect("log poisoned").push(intent);
        dnatreecalc_skin_framework::IntentReceipt::rejected(
            dnatreecalc_skin_framework::IntentError::GridEntryRejected {
                diagnostics: vec![dnatreecalc_skin_framework::GridEntryDiagnosticProjection {
                    message: "unexpected end of formula".to_string(),
                    span: Some((3, 3)),
                }],
            },
        )
    }
}

/// K2 acceptance (2), browser proof (N2(2) rerun in K context): on a
/// rejection receipt the in-grid editor stays open with the entered text
/// intact and renders the diagnostics under it — the same shared
/// `CellEntryEditor`/`EntryDiagnostics` composition N2 proved for the
/// notebook, now proven mounted inside the grid's own cell-render path.
#[wasm_bindgen_test]
async fn reject_in_grid_keeps_editor_open_and_shows_diagnostics_in_k_context() {
    let dispatcher = RejectingDispatcher::default();
    let dispatch: Arc<dyn Dispatcher> = Arc::new(dispatcher.clone());
    let host = mount_grid_fixture_with_dispatch("dtc-k2-reject", dispatch);
    next_tick().await;

    let a1: web_sys::HtmlElement = host
        .query_selector_all(".dtc-grid__cell")
        .expect("query cells")
        .item(0)
        .expect("A1 must render")
        .dyn_into()
        .expect("cell is an HtmlElement");
    a1.click();
    next_tick().await;

    let input = editor_input(&host);
    type_into(&input, "=1+");
    press_key(&input, "Enter");
    next_tick().await;

    // Exactly one dispatch, and it was rejected — the editor must stay open.
    assert_eq!(dispatcher.intents().len(), 1, "one commit attempt");
    assert!(
        query_in(&host, ".dtc-cell-entry__input").is_some(),
        "a rejection keeps the in-grid editor mounted"
    );

    // The entered text is retained (engine guaranteed no mutation).
    let input = editor_input(&host);
    assert_eq!(
        input.value(),
        "=1+",
        "the rejected text is retained in the editor buffer"
    );

    // The diagnostics render under the editor via the shared component.
    let diagnostics = query_in(&host, ".dtc-entry-diagnostics")
        .expect("the diagnostics list must render on rejection");
    let text = diagnostics.text_content().unwrap_or_default();
    assert!(
        text.contains("unexpected end of formula"),
        "the diagnostic message renders: {text}"
    );
}

/// K2 acceptance (2), browser proof (N2(3) rerun in K context): `Esc`
/// reverts the in-grid editor's buffer and dispatches nothing.
#[wasm_bindgen_test]
async fn escape_in_grid_reverts_without_dispatch_in_k_context() {
    let dispatcher = RecordingDispatcher::new();
    let dispatch: Arc<dyn Dispatcher> = Arc::new(dispatcher.clone());
    let host = mount_grid_fixture_with_dispatch("dtc-k2-escape", dispatch);
    next_tick().await;

    let a1: web_sys::HtmlElement = host
        .query_selector_all(".dtc-grid__cell")
        .expect("query cells")
        .item(0)
        .expect("A1 must render")
        .dyn_into()
        .expect("cell is an HtmlElement");
    a1.click();
    next_tick().await;

    let input = editor_input(&host);
    type_into(&input, "garbage");
    press_key(&input, "Escape");
    next_tick().await;

    assert_eq!(dispatcher.intents().len(), 0, "Esc must dispatch nothing");
    assert!(
        query_in(&host, ".dtc-cell-entry__input").is_none(),
        "Esc closes the in-grid editor back to the value display"
    );
    // The original literal (10) still renders — the buffer reverted, not the
    // typed "garbage".
    let cell_text = a1.text_content().unwrap_or_default();
    assert!(
        cell_text.contains('1') && !cell_text.contains("garbage"),
        "the cell shows the original value after Esc, not the typed text: {cell_text:?}"
    );
}

/// K2 acceptance (2), browser proof (the "clear" third of "commit/reject/
/// clear" — §C.3: "`Delete` on a selected cell -> `ClearGridCell`"): clicking
/// a cell to select it (not edit it) and pressing `Delete` on the grid
/// dispatches exactly one `ClearGridCell` at that cell, with no editor ever
/// mounted.
#[wasm_bindgen_test]
async fn delete_key_on_selected_cell_dispatches_clear_grid_cell() {
    let (host, _show_formulas, dispatcher) = mount_grid_fixture("dtc-k2-delete-clear");
    next_tick().await;

    let a1: web_sys::HtmlElement = host
        .query_selector_all(".dtc-grid__cell")
        .expect("query cells")
        .item(0)
        .expect("A1 must render")
        .dyn_into()
        .expect("cell is an HtmlElement");
    a1.click();
    next_tick().await;

    // A click on an Editable cell both selects it and opens the editor (K2's
    // click contract, `attempt_edit`). Close it with Escape (dispatches
    // nothing) so the cell is selected but not editing -- the state Delete's
    // grid-container keyboard model actually targets.
    let input = editor_input(&host);
    press_key(&input, "Escape");
    next_tick().await;
    assert!(
        query_in(&host, ".dtc-cell-entry__input").is_none(),
        "Escape closed the editor; the cell is selected but not editing"
    );

    let grid = query_in(&host, ".dtc-grid").expect("grid surface must mount");
    press_key_on(&grid, "Delete");
    next_tick().await;

    let intents = dispatcher.intents();
    let cleared: Vec<_> = intents
        .iter()
        .filter(|intent| matches!(intent, WorkspaceIntent::ClearGridCell { .. }))
        .collect();
    assert_eq!(
        cleared.len(),
        1,
        "Delete on a selected cell must dispatch exactly one ClearGridCell, got {}",
        cleared.len()
    );
    let WorkspaceIntent::ClearGridCell { row, col, .. } = cleared[0] else {
        unreachable!()
    };
    assert_eq!((*row, *col), (1, 1), "the clear targets the selected cell");
    assert!(
        query_in(&host, ".dtc-cell-entry__input").is_none(),
        "Delete never opens the editor"
    );
}

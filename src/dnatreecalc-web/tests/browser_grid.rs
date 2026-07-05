//! K1b: browser-harness proof for the grid canvas upgrades (interest
//! coalescing + authored-aware cell render + show-formulas mode).
//!
//! In its own file, not `tests/browser_smoke.rs` (H11's lane), per the K1b
//! coordination note: K1b and N1 run concurrently and both extend the H11
//! harness, so each owns a separate test file to avoid colliding edits to
//! the same file.
//!
//! These tests mount the shared `grid_canvas::grid_surface` component
//! **directly** into the browser DOM with a synthetic `WorkspaceState`
//! carrying real authored metadata, rather than going through the `?grid=1`
//! demo route: the demo grid rides the tree-model session, whose grid
//! projection deliberately fills `authored: None` (H3 scoped the
//! authored-view fill to the workbook host-core path —
//! `dnatreecalc-host/src/app/session.rs`, `grid_projection_for`), so the
//! app-route fixture can never exercise K1b's authored-aware branches. The
//! fresh-eyes review of this bead traced that gap; wiring authored fill into
//! the tree-model session is host-lane work outside K1b's file boundary and
//! is recorded with the coordinator. Mounting the component directly is
//! still a real browser proof: a live Leptos mount, real DOM, real scroll
//! events, real `requestAnimationFrame` coalescing.
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

    // Exactly one cell is marked — the two Editable cells render unmarked.
    let marked = host
        .query_selector_all("[class*='dtc-grid__cell--']")
        .expect("query affordance cells");
    assert_eq!(
        marked.length(),
        1,
        "only the SpillDisplay cell carries an affordance modifier"
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

//! Browser suite for the DNA Calc app (SHELL_SPEC §10.1-10.4, D3 parity).
//!
//! Mounts the real [`CalcApp`] over the host-core demo workbook and proves, in
//! the DOM, that the Calc composition mounts its full region set with a stage
//! switcher, that stage switching is re-projection (the incoming stage mounts
//! and reads shared continuity state), that the DEGRADE bridge edits a workbook
//! cell via `EnterGridCell` with the three-way outcome rendered honestly, and
//! that the atlas + deck open. Reserved parity/evidence slots render NOTHING.
//!
//! S3.11 registers the REAL `dnacalc_stage_sheet::SheetStage` in place of the
//! former Sheet `StubStage`: every test that used to treat
//! `[data-testid="calc-stage-sheet"]` as the "Sheet is mounted" marker now
//! reads the real stage's own root (`[data-testid="sheet-root"]`), and two new
//! tests below (`calc_sheet_stage_renders_the_canvas_grid` /
//! `calc_sheet_click_opens_editor_and_commits`) prove the canvas actually
//! draws a plan and that a click opens the overlay editor and commits a real
//! `EnterGridCell`. The Sheet stub's `.calc-stage__continuity` readout is gone
//! with it — `calc_stage_switch_reprojects_and_preserves_continuity_surface`
//! now reads that readout off the Model stub (still a `StubStage`) instead.

#![cfg(target_arch = "wasm32")]

use dnacalc_app::app::CalcApp;
use dnacalc_shell::RuntimeContext;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

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

fn mount() -> web_sys::HtmlElement {
    let document = web_sys::window().unwrap().document().unwrap();
    let host = document
        .create_element("div")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    document.body().unwrap().append_child(&host).unwrap();
    leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        view! { <CalcApp runtime=RuntimeContext::Browser /> }
    })
    .forget();
    host
}

fn query(host: &web_sys::HtmlElement, selector: &str) -> Option<web_sys::Element> {
    host.query_selector(selector).unwrap()
}

/// The notebook block (`[data-testid="notebook-block"]`) whose name row contains
/// `name_fragment` (e.g. `"R6C1"`), or `None` if no such block is rendered.
fn notebook_block(host: &web_sys::HtmlElement, name_fragment: &str) -> Option<web_sys::Element> {
    let blocks = host
        .query_selector_all("[data-testid=\"notebook-block\"]")
        .unwrap();
    for i in 0..blocks.length() {
        let block: web_sys::Element = blocks.item(i).unwrap().unchecked_into();
        let name = block
            .query_selector("[data-testid=\"notebook-block-name\"]")
            .unwrap();
        if let Some(name) = name
            && name
                .text_content()
                .unwrap_or_default()
                .contains(name_fragment)
        {
            return Some(block);
        }
    }
    None
}

/// The value-region text of the notebook block whose name contains
/// `name_fragment` (read from the block's dedicated value element, so a name
/// that happens to contain the value's digits cannot skew the assertion).
fn notebook_block_value(host: &web_sys::HtmlElement, name_fragment: &str) -> Option<String> {
    let block = notebook_block(host, name_fragment)?;
    let value = block
        .query_selector("[data-testid=\"notebook-block-value\"]")
        .unwrap()?;
    Some(value.text_content().unwrap_or_default())
}

/// The degrade textarea embedded in one specific notebook block (its OWN
/// editor), found by scoping the query to that block element — so a block's
/// editor can never be confused with the app's bridge-slot editor or another
/// block's editor.
fn block_editor(block: &web_sys::Element) -> web_sys::HtmlTextAreaElement {
    block
        .query_selector(".dna-bridge--degrade .dna-bridge__input")
        .unwrap()
        .expect("the block carries its own degrade editor")
        .unchecked_into()
}

/// Type `text` into a specific block's editor and fire input (as a keystroke
/// would), then Enter on that same editor to commit.
fn commit_block_edit(area: &web_sys::HtmlTextAreaElement, text: &str) {
    area.set_value(text);
    area.dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
    let init = web_sys::KeyboardEventInit::new();
    init.set_key("Enter");
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    let target: web_sys::EventTarget = area.clone().unchecked_into();
    target.dispatch_event(&event).unwrap();
}

/// The outcome-chip `data-outcome` label of the notebook block whose name
/// contains `name_fragment`, or `None` if the block has no committed outcome yet.
fn notebook_block_outcome(host: &web_sys::HtmlElement, name_fragment: &str) -> Option<String> {
    let block = notebook_block(host, name_fragment)?;
    let chip = block
        .query_selector("[data-testid=\"notebook-block-outcome\"]")
        .unwrap()?;
    chip.get_attribute("data-outcome")
}

fn shell_root(host: &web_sys::HtmlElement) -> web_sys::EventTarget {
    host.query_selector(".dna-shell")
        .unwrap()
        .expect("shell root mounts")
        .unchecked_into()
}

/// Set the DEGRADE editor's text and fire an input event (as a keystroke would).
fn set_degrade_text(host: &web_sys::HtmlElement, text: &str) {
    let area = query(host, ".dna-bridge--degrade .dna-bridge__input").expect("degrade textarea");
    let area: &web_sys::HtmlTextAreaElement = area.unchecked_ref();
    area.set_value(text);
    area.dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
}

/// Fire Enter on the degrade textarea (its own handler emits CommitRequested).
fn commit_degrade(host: &web_sys::HtmlElement) {
    let area = query(host, ".dna-bridge--degrade .dna-bridge__input").expect("degrade textarea");
    let init = web_sys::KeyboardEventInit::new();
    init.set_key("Enter");
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    let target: web_sys::EventTarget = area.unchecked_into();
    target.dispatch_event(&event).unwrap();
}

fn press_chord(target: &web_sys::EventTarget, key: &str, ctrl: bool, alt: bool, shift: bool) {
    let _ = press_chord_observed(target, key, ctrl, alt, shift);
}

/// §10.1 — the full Calc region set mounts (with the Registry rail and the
/// stage switcher), and the reserved parity slot renders nothing.
#[wasm_bindgen_test]
fn calc_mounts_full_region_set_with_switcher() {
    let host = mount();

    for region in [
        "mast",
        "bridge",
        "registry",
        "stage-host",
        "inspector",
        "strip",
    ] {
        assert!(
            query(&host, &format!("[data-region=\"{region}\"]")).is_some(),
            "region {region} must mount in the Calc composition"
        );
    }

    // Four stage tabs (Sheet, Model, Notebook, Atlas).
    let tabs = host.query_selector_all("[data-stage-tab]").unwrap();
    assert_eq!(
        tabs.length(),
        4,
        "the real Sheet stage + the Model stub + Notebook + Atlas compose the switcher"
    );

    let parity = query(&host, "[data-slot=\"parity\"]").expect("parity slot reserved in layout");
    assert_eq!(parity.text_content().unwrap_or_default(), "");
    assert_eq!(parity.child_element_count(), 0);
}

/// §10.1 — stage switching is re-projection: the incoming stage mounts and its
/// shared-continuity readout is present (the shared state survives the switch,
/// it is not reset to a fresh default).
///
/// RETARGETED for S3.11: the real Sheet stage (`dnacalc_stage_sheet::SheetStage`)
/// carries no `.calc-stage__continuity` readout — only the Model `StubStage`
/// still does. So the mount/gone assertions stay on the real Sheet's own root
/// (`sheet-root`), but the continuity read itself moves to Model. To keep the
/// proof non-tautological (not just "one switch, read once"), the test drives
/// a full re-projection ROUND TRIP through Model — Sheet -> Model (read
/// `before`) -> Notebook (Model torn down) -> Model again (read `after`) —
/// so equality proves the shared continuity state survives being re-projected
/// away and back, not merely that it was never touched.
#[wasm_bindgen_test]
async fn calc_stage_switch_reprojects_and_preserves_continuity_surface() {
    let host = mount();
    next_tick().await;

    // Initial: the first visible stage (the real Sheet) mounts.
    assert!(
        query(&host, "[data-testid=\"sheet-root\"]").is_some(),
        "the Sheet stage mounts first"
    );

    // Switch to Model via its mast tab (a re-projection switch).
    let model_tab = query(&host, "[data-stage-tab=\"model\"]").expect("model tab");
    let target: web_sys::EventTarget = model_tab.unchecked_into();
    target
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    next_tick().await;

    // The Model stage is now mounted, the Sheet stage gone (re-projection).
    assert!(
        query(&host, "[data-testid=\"calc-stage-model\"]").is_some(),
        "the Model stage mounts after the switch"
    );
    assert!(
        query(&host, "[data-testid=\"sheet-root\"]").is_none(),
        "switching is re-projection: only one stage mounts at a time"
    );
    let before = query(
        &host,
        "[data-testid=\"calc-stage-model\"] .calc-stage__continuity",
    )
    .and_then(|el| el.get_attribute("data-selection"))
    .expect("continuity readout on the Model stage");

    // Switch away to Notebook (tearing Model down) and then back to Model — a
    // full re-projection round trip the shared continuity state must survive.
    let notebook_tab = query(&host, "[data-stage-tab=\"notebook\"]").expect("notebook tab");
    let target: web_sys::EventTarget = notebook_tab.unchecked_into();
    target
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    next_tick().await;
    assert!(
        query(&host, "[data-testid=\"calc-stage-model\"]").is_none(),
        "switching away tears down the Model stage (re-projection)"
    );

    let model_tab = query(&host, "[data-stage-tab=\"model\"]").expect("model tab");
    let target: web_sys::EventTarget = model_tab.unchecked_into();
    target
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    next_tick().await;

    // The shared continuity state survived the round trip (same value the
    // re-mounted Model stage reads back).
    let after = query(
        &host,
        "[data-testid=\"calc-stage-model\"] .calc-stage__continuity",
    )
    .and_then(|el| el.get_attribute("data-selection"))
    .expect("continuity readout on the Model stage after the round trip");
    assert_eq!(
        before, after,
        "continuity state survives a full re-projection round trip"
    );
}

/// S2.3 — the Notebook stage (`dnacalc-stage-notebook`) is registered into the
/// Calc composition: the switcher lists a Notebook tab, and switching to it
/// re-projects the stage-host to the Notebook stage's reactive block list. (Its
/// honest-empty card is unit-tested in the notebook crate; over the demo
/// workbook the stage renders content, so this test asserts the root mounts —
/// the reactive block content is proved by `calc_notebook_renders_reactive_block_list`.)
#[wasm_bindgen_test]
async fn calc_stage_switch_to_notebook_mounts_notebook_stage() {
    let host = mount();
    next_tick().await;

    // The switcher lists a Notebook entry.
    let notebook_tab =
        query(&host, "[data-stage-tab=\"notebook\"]").expect("notebook tab listed in the switcher");

    // Initial: the first visible stage (the real Sheet) mounts.
    assert!(
        query(&host, "[data-testid=\"sheet-root\"]").is_some(),
        "the Sheet stage mounts first"
    );

    // Switch to Notebook via its mast tab (a re-projection switch).
    let target: web_sys::EventTarget = notebook_tab.unchecked_into();
    target
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    next_tick().await;

    // The Notebook stage is now mounted, the Sheet stage gone (re-projection).
    assert!(
        query(&host, "[data-stage=\"notebook\"]").is_some(),
        "the Notebook stage mounts after the switch"
    );
    assert!(
        query(&host, "[data-testid=\"notebook-root\"]").is_some(),
        "the Notebook stage's reactive block list mounts"
    );
    assert!(
        query(&host, "[data-testid=\"sheet-root\"]").is_none(),
        "switching is re-projection: only one stage mounts at a time"
    );
}

/// S2.15 — the Atlas stage (`dnacalc-stage-atlas`) is registered into the
/// Calc composition: the switcher lists an Atlas tab, and switching to it
/// re-projects the stage-host to the Atlas stage's structure map + calc HUD,
/// each derived from the real demo workbook (2 sheets, no defined names, and
/// a populated `workbook_calc` — see `dnacalc-host-core`'s
/// `snapshot_of_demo_workbook_projects_grids_values_and_calc_state`).
#[wasm_bindgen_test]
async fn calc_stage_switch_to_atlas_mounts_atlas_stage() {
    let host = mount();
    next_tick().await;

    // The switcher lists an Atlas entry.
    let atlas_tab =
        query(&host, "[data-stage-tab=\"atlas\"]").expect("atlas tab listed in the switcher");

    // Initial: the first visible stage (the real Sheet) mounts.
    assert!(
        query(&host, "[data-testid=\"sheet-root\"]").is_some(),
        "the Sheet stage mounts first"
    );

    // Switch to Atlas via its mast tab (a re-projection switch).
    let target: web_sys::EventTarget = atlas_tab.unchecked_into();
    target
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    next_tick().await;

    // The Atlas stage is now mounted, the Sheet stage gone (re-projection).
    assert!(
        query(&host, "[data-stage=\"atlas\"]").is_some(),
        "the Atlas stage mounts after the switch"
    );
    assert!(
        query(&host, "[data-testid=\"atlas-root\"]").is_some(),
        "the Atlas stage's root mounts"
    );
    assert!(
        query(&host, "[data-testid=\"sheet-root\"]").is_none(),
        "switching is re-projection: only one stage mounts at a time"
    );

    // The structure map lists the demo workbook's 2 real sheets.
    let structure_map =
        query(&host, "[data-testid=\"atlas-structure-map\"]").expect("structure map mounts");
    let sheet_rows = structure_map
        .query_selector_all("[data-testid=\"atlas-sheet\"]")
        .unwrap();
    assert_eq!(sheet_rows.length(), 2, "the demo workbook has 2 sheets");
    let sheet_names: Vec<String> = (0..sheet_rows.length())
        .map(|i| {
            sheet_rows
                .item(i)
                .unwrap()
                .text_content()
                .unwrap_or_default()
                .trim()
                .to_string()
        })
        .collect();
    assert!(
        sheet_names.contains(&"Sheet1".to_string()),
        "Sheet1 is listed, got {sheet_names:?}"
    );
    assert!(
        sheet_names.contains(&"Sheet2".to_string()),
        "Sheet2 is listed, got {sheet_names:?}"
    );

    // The demo authors no defined names, so the structure map shows the honest
    // empty note (never a placeholder row).
    assert!(
        query(&host, "[data-testid=\"atlas-names-empty\"]").is_some(),
        "the demo has no defined names, so the honest-empty note renders"
    );

    // The calc HUD mounts the POPULATED branch (the demo snapshot fills
    // `workbook_calc`): assert a Present-only element (`atlas-hud-mode`), not the
    // shared `atlas-hud` container — which both the Empty and Present branches
    // emit, so it would pass even if the wrong branch rendered.
    assert!(
        query(&host, "[data-testid=\"atlas-hud-mode\"]").is_some(),
        "the populated calc HUD renders its mode row (not the empty note)"
    );
}

/// S2.5 — the Notebook stage renders the reactive single-column block list from
/// host truth, and re-derives it when the workspace changes.
///
/// The demo workbook authors no defined names or tables, so every entry is a
/// Cell block (the escape-hatch idiom) named by its grid address — e.g. Sheet1
/// `A3` renders as a block whose name row contains `R3C1` and whose value region
/// reads `3` (`R3C1` is unique across the two demo sheets; `R1C1`/`R1C2` appear
/// on both). That proves render-from-host-truth for the seeded content.
///
/// REACTIVITY is driven through the app's existing DEGRADE bridge path (the same
/// path `calc_degrade_edits_cell_with_honest_three_way_outcome` drives), which
/// authors Sheet1 `A6` (row 6, col 1) — empty in the demo, so no block exists
/// for it initially. Committing the literal `42` makes a new Cell block appear
/// (`free value` classification, value `42`); committing the formula `=A1+A5`
/// into the SAME cell re-renders that block with value `6` and reclassifies it
/// `output`. Because the notebook body is a closure over `ctx.workspace`, the
/// mounted stage re-derives both times with no remount — the block list tracks
/// host truth. A6 is chosen because it is the only cell the fixed-target bridge
/// can drive, and because going empty→literal→formula exercises both a new-block
/// derivation and an in-place value+classification change on one stable block.
#[wasm_bindgen_test]
async fn calc_notebook_renders_reactive_block_list() {
    let host = mount();
    next_tick().await;

    // Switch to the Notebook stage.
    let notebook_tab = query(&host, "[data-stage-tab=\"notebook\"]").expect("notebook tab");
    let target: web_sys::EventTarget = notebook_tab.unchecked_into();
    target
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    next_tick().await;

    // The reactive block list renders (not the honest-empty card): the demo's
    // seeded cells derive as blocks.
    query(&host, "[data-testid=\"notebook-root\"]").expect("notebook root mounts");
    let blocks = host
        .query_selector_all("[data-testid=\"notebook-block\"]")
        .unwrap();
    assert!(
        blocks.length() >= 1,
        "the demo's seeded cells render as blocks, got {}",
        blocks.length()
    );
    assert!(
        query(&host, "[data-testid=\"notebook-empty\"]").is_none(),
        "not the empty card when the demo has content"
    );

    // A real block from the demo: Sheet1 A3 (a literal `3`). The name row names
    // the cell (`R3C1`, unique across both demo sheets) and the value region
    // reads its computed value `3`.
    let a3_value = notebook_block_value(&host, "R3C1").expect("a block names Sheet1 A3");
    assert_eq!(
        a3_value.trim(),
        "3",
        "Sheet1 A3's block value region reads 3"
    );
    // Each block carries its classification chip.
    assert!(
        query(&host, "[data-testid=\"notebook-classification\"]").is_some(),
        "blocks carry a classification chip"
    );

    // A6 is empty in the demo: no block for it yet.
    assert!(
        notebook_block_value(&host, "R6C1").is_none(),
        "A6 is empty in the demo — no block for it yet"
    );

    // Drive reactivity via the existing bridge path: author A6 as literal `42`.
    set_degrade_text(&host, "42");
    next_tick().await;
    commit_degrade(&host);
    next_tick().await;

    // The reactive closure re-derived: a new A6 block appeared with value `42`,
    // classified `free_value` (a literal nothing consumes).
    let a6_block = notebook_block(&host, "R6C1").expect("A6 block appears after authoring it");
    assert_eq!(
        a6_block.get_attribute("data-block-kind").as_deref(),
        Some("cell"),
        "A6 is a Cell block (the escape-hatch idiom)"
    );
    assert_eq!(
        a6_block.get_attribute("data-classification").as_deref(),
        Some("free_value"),
        "A6 literal classifies as free_value"
    );
    assert_eq!(
        notebook_block_value(&host, "R6C1")
            .as_deref()
            .map(str::trim),
        Some("42"),
        "A6 block's value region shows the literal 42"
    );

    // Author the SAME cell as a formula (=A1+A5 = 6 over the demo). The same
    // block re-renders with the new value and classification — proof the value
    // region tracks host truth, not a one-shot snapshot.
    set_degrade_text(&host, "=A1+A5");
    next_tick().await;
    commit_degrade(&host);
    next_tick().await;

    let a6_block = notebook_block(&host, "R6C1").expect("A6 block still present");
    assert_eq!(
        a6_block.get_attribute("data-classification").as_deref(),
        Some("output"),
        "A6 reclassifies literal (free_value) -> formula (output)"
    );
    assert_eq!(
        notebook_block_value(&host, "R6C1")
            .as_deref()
            .map(str::trim),
        Some("6"),
        "A6 block's value region now shows the formula result 6"
    );
}

/// S2.6 — a notebook Cell block is EDITABLE through its OWN embedded degrade
/// editor: typing a formula into the block for Sheet1 `A3` (R3C1, value 3) and
/// committing dispatches `EnterGridCell` built from THAT block's own row/col, so
/// the target block updates (value 6, outcome chip `formula`) while a SIBLING
/// block — Sheet1 `A2` (R2C1, value 2) — is left completely untouched. This is
/// the wrong-cell guard in the DOM: the critique's key risk is a block writing
/// to the wrong cell, so the test asserts BOTH the target changed AND the
/// sibling did not.
#[wasm_bindgen_test]
async fn calc_notebook_block_edit_writes_its_own_cell_not_a_sibling() {
    let host = mount();
    next_tick().await;

    // Switch to the Notebook stage.
    let notebook_tab = query(&host, "[data-stage-tab=\"notebook\"]").expect("notebook tab");
    let target: web_sys::EventTarget = notebook_tab.unchecked_into();
    target
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    next_tick().await;

    // Before: R3C1 (A3) reads 3, its sibling R2C1 (A2) reads 2, and neither has
    // a committed outcome yet.
    assert_eq!(
        notebook_block_value(&host, "R3C1")
            .as_deref()
            .map(str::trim),
        Some("3"),
        "Sheet1 A3 seeds as 3"
    );
    assert_eq!(
        notebook_block_value(&host, "R2C1")
            .as_deref()
            .map(str::trim),
        Some("2"),
        "Sheet1 A2 (the sibling) seeds as 2"
    );
    assert!(
        notebook_block_outcome(&host, "R3C1").is_none(),
        "no outcome chip before the block is committed"
    );

    // Edit + commit through the R3C1 block's OWN editor.
    let r3c1 = notebook_block(&host, "R3C1").expect("the R3C1 block renders");
    commit_block_edit(&block_editor(&r3c1), "=A1+A5");
    next_tick().await;

    // The target block updated: value 6 (A1 + A5 = 1 + 5) and outcome `formula`.
    assert_eq!(
        notebook_block_value(&host, "R3C1")
            .as_deref()
            .map(str::trim),
        Some("6"),
        "the edited block's value region now shows the formula result 6"
    );
    assert_eq!(
        notebook_block_outcome(&host, "R3C1").as_deref(),
        Some("formula"),
        "the edited block shows the host's three-way outcome (formula)"
    );

    // The sibling block is untouched — the commit hit R3C1, not R2C1.
    assert_eq!(
        notebook_block_value(&host, "R2C1")
            .as_deref()
            .map(str::trim),
        Some("2"),
        "the sibling block R2C1 is unchanged by the R3C1 commit"
    );
    assert!(
        notebook_block_outcome(&host, "R2C1").is_none(),
        "the sibling block never committed, so it carries no outcome chip"
    );
}

/// Type `text` into a plain `<input>` element and fire an `input` event (as a
/// keystroke would).
fn type_into_input(input: &web_sys::HtmlInputElement, text: &str) {
    input.set_value(text);
    input
        .dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
}

/// Click a plain DOM element (a real `MouseEvent`, as a user click would fire).
fn click(element: &web_sys::Element) {
    element
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
}

/// S2.8 — the Notebook's name-first authoring affordance: revealing the `+
/// name` form, filling `GrowthRate` / `0.12`, and clicking Create dispatches
/// the atomic `WorkspaceIntent::CreateNamedValue` (through the exact same
/// `ctx.dispatch` / workbook-dispatcher path `calc_notebook_renders_reactive_block_list`
/// drives for ordinary cell commits), and the resulting workbook-scoped name
/// appears as a real `Name` block in the reactive list — no app-level refresh
/// needed, the workbook dispatcher's republish is what repaints it.
#[wasm_bindgen_test]
async fn calc_notebook_create_name_affordance_adds_a_name_block() {
    let host = mount();
    next_tick().await;

    // Switch to the Notebook stage.
    let notebook_tab = query(&host, "[data-stage-tab=\"notebook\"]").expect("notebook tab");
    let target: web_sys::EventTarget = notebook_tab.unchecked_into();
    target
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    next_tick().await;

    // No GrowthRate block yet.
    assert!(
        notebook_block(&host, "GrowthRate").is_none(),
        "GrowthRate is not yet defined"
    );

    // The `+ name` control reveals the form.
    let add_name_toggle =
        query(&host, "[data-testid=\"notebook-add-name\"]").expect("the + name control mounts");
    assert!(
        query(&host, "[data-testid=\"notebook-name-input\"]").is_none(),
        "the name/value inputs are not shown until the affordance is opened"
    );
    click(&add_name_toggle);
    next_tick().await;

    let name_input = query(&host, "[data-testid=\"notebook-name-input\"]")
        .expect("the name input reveals")
        .unchecked_into::<web_sys::HtmlInputElement>();
    let value_input = query(&host, "[data-testid=\"notebook-name-value\"]")
        .expect("the value input reveals")
        .unchecked_into::<web_sys::HtmlInputElement>();
    let create_button =
        query(&host, "[data-testid=\"notebook-create-name\"]").expect("the Create button reveals");

    type_into_input(&name_input, "GrowthRate");
    type_into_input(&value_input, "0.12");
    click(&create_button);
    next_tick().await;

    // A real Name block now renders — kind=name, carrying the name text.
    let block = notebook_block(&host, "GrowthRate").expect("a Name block appears for GrowthRate");
    assert_eq!(
        block.get_attribute("data-block-kind").as_deref(),
        Some("name"),
        "GrowthRate derives as a Name block, not a Cell block"
    );

    // The inputs cleared after a successful commit.
    let name_input = query(&host, "[data-testid=\"notebook-name-input\"]")
        .expect("the name input is still mounted")
        .unchecked_into::<web_sys::HtmlInputElement>();
    assert_eq!(name_input.value(), "", "the name input clears after Create");
}

/// S2.8 — an empty name is an honest no-op: the Create button is disabled and
/// clicking it (or leaving the name blank) dispatches nothing at all — no
/// fabricated name is ever created.
#[wasm_bindgen_test]
async fn calc_notebook_create_name_empty_name_is_honest_no_op() {
    let host = mount();
    next_tick().await;

    let notebook_tab = query(&host, "[data-stage-tab=\"notebook\"]").expect("notebook tab");
    let target: web_sys::EventTarget = notebook_tab.unchecked_into();
    target
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    next_tick().await;

    let add_name_toggle =
        query(&host, "[data-testid=\"notebook-add-name\"]").expect("the + name control mounts");
    click(&add_name_toggle);
    next_tick().await;

    let blocks_before = host
        .query_selector_all("[data-testid=\"notebook-block\"]")
        .unwrap()
        .length();

    // Leave the name blank; only fill the value. The Create button must be
    // disabled and an inline hint must explain why.
    let value_input = query(&host, "[data-testid=\"notebook-name-value\"]")
        .expect("the value input reveals")
        .unchecked_into::<web_sys::HtmlInputElement>();
    type_into_input(&value_input, "0.5");
    next_tick().await;

    let create_button = query(&host, "[data-testid=\"notebook-create-name\"]")
        .expect("the Create button reveals")
        .unchecked_into::<web_sys::HtmlButtonElement>();
    assert!(
        create_button.disabled(),
        "Create is disabled with an empty name"
    );
    let hint =
        query(&host, "[data-testid=\"notebook-add-name-hint\"]").expect("the hint element mounts");
    assert!(
        !hint.text_content().unwrap_or_default().trim().is_empty(),
        "an inline hint explains the disabled state"
    );

    // Clicking the disabled button anyway dispatches nothing: no new block
    // appears, the count is unchanged.
    click(&create_button);
    next_tick().await;
    let blocks_after = host
        .query_selector_all("[data-testid=\"notebook-block\"]")
        .unwrap()
        .length();
    assert_eq!(
        blocks_after, blocks_before,
        "an empty name never fabricates a new block"
    );
}

/// S2.9 — the reviewer-persona read-only render (NOTEBOOK_SPEC §6 scenario 4):
/// when the governing persona is switched away from Author, the Notebook stage
/// renders the SAME content fully readable but with ZERO enabled mutation
/// controls — this IS the report artifact (there is no separate export in v1).
///
/// As Author the demo's editable cells each carry their OWN degrade editor
/// (`notebook-block-edit`) and the `+ name` authoring control is present. After
/// dispatching `SetPersona { Reviewer }` through the app dispatcher (driven the
/// real way — clicking the command deck's `shell.persona.reviewer` entry, which
/// dispatches through the app's `PersonaDispatcher` and writes
/// `SharedSkinState.persona`), the reactive persona gate repaints the
/// still-mounted stage: EVERY per-block editor is gone and the `+ name` control
/// is absent, yet the content stays readable — a value region still renders and
/// the known demo block Sheet1 `A3` (R3C1, value `3`) still shows its name and
/// value. This proves the gate is REACTIVE (a runtime `SetPersona` flips the
/// surface, not just a mount-time snapshot) and HONEST (no disabled-looking
/// editor lingers). It fails against the pre-guard code, whose always-on editors
/// keep `notebook-block-edit` present after the switch.
#[wasm_bindgen_test]
async fn calc_notebook_reviewer_persona_renders_read_only() {
    let host = mount();
    next_tick().await;

    // Switch to the Notebook stage.
    let notebook_tab = query(&host, "[data-stage-tab=\"notebook\"]").expect("notebook tab");
    let target: web_sys::EventTarget = notebook_tab.unchecked_into();
    target
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    next_tick().await;

    // As Author (the default persona): editable blocks carry their own editors,
    // and the `+ name` authoring control is present.
    let editors_as_author = host
        .query_selector_all("[data-testid=\"notebook-block-edit\"]")
        .unwrap()
        .length();
    assert!(
        editors_as_author >= 1,
        "as Author the demo's editable cells each render their own editor, got {editors_as_author}"
    );
    assert!(
        query(&host, "[data-testid=\"notebook-add-name\"]").is_some(),
        "as Author the `+ name` authoring control is present"
    );
    // The readable content that must SURVIVE the persona flip: Sheet1 A3 reads 3.
    assert_eq!(
        notebook_block_value(&host, "R3C1")
            .as_deref()
            .map(str::trim),
        Some("3"),
        "Sheet1 A3's value region reads 3 as Author"
    );

    // Switch persona to Reviewer the real way: open the command deck (Ctrl+K) and
    // click its `shell.persona.reviewer` entry, dispatching `SetPersona`.
    press_chord(&shell_root(&host), "k", true, false, false);
    next_tick().await;
    let reviewer_command = query(&host, "[data-command-id=\"shell.persona.reviewer\"]")
        .expect("the command deck lists the reviewer persona switch");
    click(&reviewer_command);
    next_tick().await;

    // The reactive persona gate repainted the still-mounted Notebook: ZERO
    // per-block editors remain.
    let editors_as_reviewer = host
        .query_selector_all("[data-testid=\"notebook-block-edit\"]")
        .unwrap()
        .length();
    assert_eq!(
        editors_as_reviewer, 0,
        "a Reviewer persona renders ZERO per-block editors (report artifact)"
    );
    // And no `+ name` authoring control — the cleanest zero-mutation render omits
    // it entirely rather than showing a disabled shell.
    assert!(
        query(&host, "[data-testid=\"notebook-add-name\"]").is_none(),
        "a Reviewer persona is offered no `+ name` authoring control"
    );

    // Content stays fully READABLE: value regions still render, and the known
    // demo block (R3C1) still shows its name and its value 3.
    assert!(
        query(&host, "[data-testid=\"notebook-block-value\"]").is_some(),
        "the value regions still render read-only for a Reviewer"
    );
    assert!(
        notebook_block(&host, "R3C1").is_some(),
        "the R3C1 block still renders — content is readable"
    );
    assert_eq!(
        notebook_block_value(&host, "R3C1")
            .as_deref()
            .map(str::trim),
        Some("3"),
        "R3C1's value region still reads 3 for a Reviewer"
    );
}

/// dtc-mohs (S2.14) — cross-stage continuity, Notebook-specific: `SharedSkinState`
/// survives a Sheet -> Notebook -> Sheet round trip, proving that switching
/// into AND out of the real (non-stub) Notebook stage is re-projection —
/// shared continuity state is carried across, never reset.
///
/// GENERIC MECHANISM ALREADY PROVEN NATIVELY: `dnacalc-shell/src/stage.rs`'s
/// `continuity_state_survives_a_stage_switch` seeds `selection_set`,
/// `focus_key`, `collapsed_keys`, and `pinned_keys` directly on a
/// `SharedSkinState`, calls `switch_stage`, and asserts every field is
/// unchanged (and `switch_stage_writes_only_the_audited_active_lens_change`
/// proves the ONLY write `switch_stage` makes is `SetActiveLens` — nothing
/// else in `SharedSkinState` is ever touched by a switch). This browser test
/// is the Notebook-SPECIFIC, real-app integration proof: it drives the switch
/// through the actual mounted app and the actual Notebook stage, not a bare
/// `switch_stage` call over a hand-built registry.
///
/// WHY PERSONA, NOT SELECTION/FOCUS: the shipped Calc app
/// (`dnacalc-app::app::CalcApp`) has no UI affordance that sets
/// `selection_set` or `focus_key` — the `StubStage`s (Sheet/Model) only ever
/// READ those fields for their continuity readout
/// (`.calc-stage__continuity`), and the Notebook stage surfaces no
/// selection/focus-setting control either. Searching every real (non-test)
/// `shared.apply` call site in `dnacalc-shell` turns up exactly four
/// `SharedStateChange` variants ever written by shipped UI:
/// `SetActiveLens` (the stage switch itself), `SetManualRecalcPending`,
/// `SetKeybindingOverrides`, and `SetPersona`. `persona` is the one
/// continuity field in that list living in the SAME `SharedSkinState` struct
/// as selection/focus, settable the real way: Ctrl+K -> the command deck's
/// `shell.persona.reviewer` entry, dispatching `SetPersona` through
/// `PersonaDispatcher` exactly as `calc_notebook_reviewer_persona_renders_read_only`
/// (S2.9) drives it. Because it lives in the identical handle and rides the
/// identical `switch_stage` re-projection path, its survival is evidence of
/// the SAME mechanism selection/focus depend on — this test plus the native
/// one together give honest browser+native coverage of continuity across the
/// Notebook (the plan's critique was that persona-for-Notebook was asserted
/// but selection/focus-survives-Notebook was not; the native test closes the
/// general case, this one closes the Notebook-specific case with the field
/// the shipped app can actually seed).
///
/// OBSERVABLE: the mast's persona chip (`.dna-mast__persona[data-persona]`,
/// `dnacalc-shell/src/shell.rs`) lives in the shell chrome, not inside any
/// stage — so it renders identically whether Sheet or Notebook is mounted,
/// letting the SAME element be read before the switch (on Sheet), mid-switch
/// (on Notebook), and after switching back (on Sheet). A second, stronger
/// signal is also asserted: because persona is seeded BEFORE the Notebook is
/// ever mounted (unlike S2.9, which flips persona while already on the
/// Notebook), the Notebook's own reviewer read-only gate (zero
/// `notebook-block-edit` editors, no `+ name` control) must already hold the
/// instant it mounts — proof the stage reads the LIVE shared handle on
/// arrival, not a fresh default it would show if the switch had reset state.
#[wasm_bindgen_test]
async fn calc_selection_focus_survives_notebook_round_trip() {
    let host = mount();
    next_tick().await;

    fn persona_chip(host: &web_sys::HtmlElement) -> String {
        query(host, ".dna-mast__persona")
            .and_then(|el| el.get_attribute("data-persona"))
            .expect("the mast's persona chip mounts (shell chrome, present regardless of stage)")
    }

    // Sanity: Author is the default persona before any seeding.
    assert_eq!(
        persona_chip(&host),
        "author",
        "Author is the default persona before seeding"
    );

    // We start on the Sheet stage.
    assert!(
        query(&host, "[data-testid=\"sheet-root\"]").is_some(),
        "the Sheet stage mounts first"
    );

    // Seed a NON-DEFAULT shared-continuity value through the shipped UI, on
    // the Sheet stage, BEFORE the Notebook is ever mounted: switch persona to
    // Reviewer via the command deck (Ctrl+K -> `shell.persona.reviewer`).
    press_chord(&shell_root(&host), "k", true, false, false);
    next_tick().await;
    let reviewer_command = query(&host, "[data-command-id=\"shell.persona.reviewer\"]")
        .expect("the command deck lists the reviewer persona switch");
    click(&reviewer_command);
    next_tick().await;

    let before = persona_chip(&host);
    assert_eq!(
        before, "reviewer",
        "the seeded value is non-trivial: reviewer, not the untouched default"
    );

    // Switch Sheet -> Notebook via its mast tab (a re-projection switch).
    let notebook_tab = query(&host, "[data-stage-tab=\"notebook\"]").expect("notebook tab");
    click(&notebook_tab);
    next_tick().await;

    // The Notebook stage is now mounted, the Sheet stage gone (re-projection).
    assert!(
        query(&host, "[data-stage=\"notebook\"]").is_some(),
        "the Notebook stage mounts after the switch"
    );
    assert!(
        query(&host, "[data-testid=\"sheet-root\"]").is_none(),
        "switching is re-projection: only one stage mounts at a time"
    );

    // MID: the seeded shared value survived entering the Notebook, unchanged.
    let mid = persona_chip(&host);
    assert_eq!(mid, before, "persona survives the Sheet -> Notebook switch");

    // Stronger signal: the Notebook's OWN reviewer read-only gate already
    // holds on arrival (persona was seeded before the mount, not after) —
    // proof the Notebook stage reads the live shared handle, not a snapshot
    // that would show Author's always-on editors if the switch had reset it.
    assert_eq!(
        host.query_selector_all("[data-testid=\"notebook-block-edit\"]")
            .unwrap()
            .length(),
        0,
        "the Notebook renders the reviewer read-only gate immediately on arrival"
    );
    assert!(
        query(&host, "[data-testid=\"notebook-add-name\"]").is_none(),
        "a Reviewer arriving on the Notebook is offered no `+ name` authoring control"
    );

    // Switch Notebook -> Sheet via its mast tab (the return leg).
    let sheet_tab = query(&host, "[data-stage-tab=\"sheet\"]").expect("sheet tab");
    click(&sheet_tab);
    next_tick().await;

    // The Sheet stage is mounted again, the Notebook stage gone.
    assert!(
        query(&host, "[data-testid=\"sheet-root\"]").is_some(),
        "the Sheet stage mounts again after switching back"
    );
    assert!(
        query(&host, "[data-stage=\"notebook\"]").is_none(),
        "switching is re-projection: the Notebook stage is gone after switching away"
    );

    // AFTER: the seeded shared value survived the FULL Sheet -> Notebook ->
    // Sheet round trip — identical before, mid, and after. A regression that
    // reset shared state on a Notebook switch (either leg) would turn this red.
    let after = persona_chip(&host);
    assert_eq!(
        after, before,
        "persona survives the full Sheet -> Notebook -> Sheet round trip"
    );
    assert_eq!(
        after, mid,
        "persona is identical before, mid, and after the round trip"
    );
}

/// §10.2 — the DEGRADE bridge edits a workbook cell via `EnterGridCell`, and
/// the host's three-way outcome (literal / formula / cleared) plus the typed
/// rejection are each rendered honestly from the receipt.
#[wasm_bindgen_test]
async fn calc_degrade_edits_cell_with_honest_three_way_outcome() {
    let host = mount();
    next_tick().await;

    // The bridge mounts in DEGRADE mode (no fake token colors).
    assert!(
        query(&host, ".dna-bridge--degrade").is_some(),
        "Calc mounts the DEGRADE bridge (pre-G1 honest path)"
    );
    assert!(
        query(
            &host,
            ".dna-bridge--degrade .dna-bridge__seg--role-function"
        )
        .is_none(),
        "degrade mode never paints token-role classes"
    );

    // Literal.
    set_degrade_text(&host, "42");
    next_tick().await;
    commit_degrade(&host);
    next_tick().await;
    let outcome = query(&host, "[data-testid=\"calc-outcome\"]").expect("outcome renders");
    assert_eq!(
        outcome.get_attribute("data-outcome").as_deref(),
        Some("literal")
    );

    // Formula (=A1+A5 over the demo workbook = 6).
    set_degrade_text(&host, "=A1+A5");
    next_tick().await;
    commit_degrade(&host);
    next_tick().await;
    let outcome = query(&host, "[data-testid=\"calc-outcome\"]").expect("outcome renders");
    assert_eq!(
        outcome.get_attribute("data-outcome").as_deref(),
        Some("formula")
    );

    // Cleared (empty commit).
    set_degrade_text(&host, "");
    next_tick().await;
    commit_degrade(&host);
    next_tick().await;
    let outcome = query(&host, "[data-testid=\"calc-outcome\"]").expect("outcome renders");
    assert_eq!(
        outcome.get_attribute("data-outcome").as_deref(),
        Some("cleared")
    );

    // Rejected (an unparseable formula) — typed rejection surfaces.
    set_degrade_text(&host, "=1+");
    next_tick().await;
    commit_degrade(&host);
    next_tick().await;
    let outcome = query(&host, "[data-testid=\"calc-outcome\"]").expect("outcome renders");
    assert_eq!(
        outcome.get_attribute("data-outcome").as_deref(),
        Some("rejected")
    );
    assert!(
        query(&host, ".dna-bridge__rejection").is_some(),
        "the degrade editor underlines the entry rejection"
    );
}

/// §10.3 — the keyboard atlas opens from the live registry.
#[wasm_bindgen_test]
async fn calc_atlas_opens_from_registry() {
    let host = mount();
    next_tick().await;
    press_chord(&shell_root(&host), "/", true, false, false);
    next_tick().await;
    let atlas = query(&host, "[data-overlay=\"keyboard-atlas\"]").expect("atlas overlay opens");
    assert!(atlas.query_selector("[data-atlas-row]").unwrap().is_some());
}

/// §10.4 — the command deck opens and lists at least 15 commands, and goto
/// recognizes an A1 address.
#[wasm_bindgen_test]
async fn calc_command_deck_lists_commands_and_goto() {
    let host = mount();
    next_tick().await;
    press_chord(&shell_root(&host), "k", true, false, false);
    next_tick().await;
    let deck = query(&host, "[data-overlay=\"command-deck\"]").expect("command deck opens");
    let rows = deck.query_selector_all("[data-command-id]").unwrap();
    assert!(
        rows.length() >= 15,
        "the deck surfaces >= 15 commands, got {}",
        rows.length()
    );

    // Goto: an A1 address is recognized as a navigation entry.
    let input = query(&host, ".dna-deck__input").expect("deck input");
    let input: &web_sys::HtmlInputElement = input.unchecked_ref();
    input.set_value("A1");
    input
        .dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
    next_tick().await;
    assert!(
        query(&host, "[data-command-id=\"goto:a1:A1\"]").is_some(),
        "the deck recognizes an A1 goto address"
    );
}

/// S3.11 — the real `dnacalc_stage_sheet::SheetStage` renders its canvas grid
/// over the host-core demo workbook. Sheet is the DEFAULT stage, so it is
/// already mounted; this asserts the canvas element is present AND that the
/// visually-hidden debug readout (`sheet-render-plan`, which mirrors exactly
/// what the redraw effect drew — see `dnacalc-stage-sheet/src/lib.rs`)
/// reports a non-zero cell count and a non-empty extent. Reading the readout
/// rather than canvas pixels catches a blank/0-size render (a canvas that got
/// no size, or a redraw effect that never ran) without a screenshot
/// assertion — the crate's own doctrine (Foundation: no screenshot
/// assertions).
#[wasm_bindgen_test]
async fn calc_sheet_stage_renders_the_canvas_grid() {
    let host = mount();
    // The redraw effect runs after layout (it reads `clientWidth`/`clientHeight`
    // off the mounted canvas) — give it generous ticks before asserting.
    next_tick().await;
    next_tick().await;
    next_tick().await;

    assert!(
        query(&host, "[data-testid=\"sheet-root\"]").is_some(),
        "the Sheet stage mounts by default"
    );
    assert!(
        query(&host, "[data-testid=\"sheet-canvas\"]").is_some(),
        "the real canvas mounts"
    );

    let plan = query(&host, "[data-testid=\"sheet-render-plan\"]")
        .expect("the debug readout mirroring the drawn RenderPlan mounts");
    let cell_count: usize = plan
        .get_attribute("data-cell-count")
        .expect("data-cell-count is set")
        .parse()
        .expect("data-cell-count is a plain integer");
    assert!(
        cell_count > 0,
        "the demo grid (Sheet1 A1..B5) windows real cells — the canvas got a \
         real size and the redraw effect drew the plan, got cell_count={cell_count}"
    );
    let extent = plan
        .get_attribute("data-extent")
        .expect("data-extent is set");
    assert!(
        !extent.is_empty(),
        "the extent readout reports the demo grid's real row x col extent, got {extent:?}"
    );
}

/// S3.11 / S3.8 — clicking the canvas at a cell's own pixel SELECTS that cell
/// (hit-tested through the SAME [`GridMetrics::default`] + origin `Viewport` the
/// redraw effect draws with — see `dnacalc-stage-sheet/src/geometry.rs`), then an
/// EDIT gesture (F2) opens the ONE overlay editor at that cell, and committing
/// through it runs the real `EnterGridCell` path end to end.
///
/// Updated at S3.8 for the SELECT-vs-EDIT split: the single click no longer opens
/// the editor (that was S3.6's conflated "selected == editing"), so the test now
/// clicks to SELECT, asserts the editor is still absent, then presses F2 to enter
/// EDIT — preserving this test's intent (a real cell edit commits end to end).
///
/// Cell (row=1, col=1) = Sheet1 `A1` (a literal `1` in the demo workbook, see
/// `dnacalc-host-core/src/demo.rs`) sits at the default metrics'
/// `cell_rect(1, 1)` = `(header_w=48, header_h=22, col_width=80,
/// row_height=22)`, so its center in the canvas's own coordinate space is
/// `(48 + 40, 22 + 11) = (88, 33)`. A dispatched `MouseEvent`'s
/// `offsetX`/`offsetY` (what the click handler reads) are computed by the
/// browser from `clientX`/`clientY` relative to the event's target's real
/// `getBoundingClientRect()` — so `clientX`/`clientY` are built by translating
/// `(88, 33)` by the canvas's actual on-screen origin, which works
/// regardless of the canvas's rendered size (this offset math depends only
/// on the canvas's top-left position, not its width/height).
///
/// COMMIT SIGNAL: an accepted commit closes the overlay (only a `Rejected`
/// outcome keeps it open with the typed diagnostics underlined — see
/// `SheetStage::mount`'s `on_event` handler in
/// `dnacalc-stage-sheet/src/lib.rs`), so the overlay's disappearance after
/// Enter is the honest, host-truth-driven proof the commit was accepted —
/// not a fabricated pass.
#[wasm_bindgen_test]
async fn calc_sheet_click_opens_editor_and_commits() {
    let host = mount();
    next_tick().await;
    next_tick().await;
    next_tick().await;

    let canvas = query(&host, "[data-testid=\"sheet-canvas\"]").expect("the canvas mounts");

    assert!(
        query(&host, "[data-testid=\"sheet-cell-editor\"]").is_none(),
        "no cell is selected before any click"
    );

    // Translate cell (1, 1)'s center — (88, 33) in the canvas's own coordinate
    // space — into viewport `clientX`/`clientY` via the canvas's real
    // bounding rect, so the browser's offsetX/offsetY computation lands on
    // the same point the stage's click handler hit-tests against.
    let rect = canvas.get_bounding_client_rect();
    let client_x = (rect.x() + 88.0).round() as i32;
    let client_y = (rect.y() + 33.0).round() as i32;

    let init = web_sys::MouseEventInit::new();
    init.set_client_x(client_x);
    init.set_client_y(client_y);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = web_sys::MouseEvent::new_with_mouse_event_init_dict("mousedown", &init)
        .expect("construct the mousedown event");
    let canvas_target: web_sys::EventTarget = canvas.clone().unchecked_into();
    canvas_target.dispatch_event(&event).unwrap();
    next_tick().await;

    // S3.8 SELECT vs EDIT: a single click SELECTS the cell (canvas highlight),
    // it does NOT open the editor. The overlay stays absent until an EDIT gesture
    // (F2 / Enter / a printable key / double-click).
    assert!(
        query(&host, "[data-testid=\"sheet-cell-editor\"]").is_none(),
        "a single click selects the cell (highlight) but does NOT open the editor"
    );

    // Press F2 to enter EDIT at the selected cell (A1). Dispatched on the sheet
    // section (`data-testid=\"sheet-root\"`, `tabindex=0`) that carries the
    // keydown grammar; its target is the <section>, not a text-entry element, so
    // the SELECT grammar's text-entry guard lets it through.
    let root = query(&host, "[data-testid=\"sheet-root\"]").expect("the sheet root mounts");
    let f2_init = web_sys::KeyboardEventInit::new();
    f2_init.set_key("F2");
    f2_init.set_bubbles(true);
    f2_init.set_cancelable(true);
    let f2_event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &f2_init)
        .expect("construct the F2 keydown event");
    let root_target: web_sys::EventTarget = root.clone().unchecked_into();
    root_target.dispatch_event(&f2_event).unwrap();
    next_tick().await;

    // The overlay opened at exactly A1 (row 1, col 1): editable (a plain
    // literal cell in the demo, not a read-only role), seeded from its OWN
    // authored text (the literal `1`, never the computed value).
    let editor = query(&host, "[data-testid=\"sheet-cell-editor\"]")
        .expect("F2 opens the ONE overlay editor at the selected cell A1");
    assert_eq!(
        editor.get_attribute("data-editable").as_deref(),
        Some("true"),
        "A1 is a plain editable literal cell in the demo workbook"
    );
    assert_eq!(
        editor.get_attribute("data-cell").as_deref(),
        Some("1:1"),
        "the editor targets exactly the clicked cell (row 1, col 1 = A1)"
    );
    let area = editor
        .query_selector(".dna-bridge--degrade .dna-bridge__input")
        .unwrap()
        .expect("the degrade editor mounts inside the overlay")
        .unchecked_into::<web_sys::HtmlTextAreaElement>();
    assert_eq!(
        area.value(),
        "1",
        "the editor seeds A1's own authored text (the demo's literal 1)"
    );

    // Commit a new literal through the editor — the real EnterGridCell path
    // (`dnacalc_stage_sheet::edit::enter_cell_intent`), the same commit seam
    // `calc_degrade_edits_cell_with_honest_three_way_outcome` proves for the
    // app's bridge slot.
    area.set_value("99");
    area.dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
    let commit_init = web_sys::KeyboardEventInit::new();
    commit_init.set_key("Enter");
    commit_init.set_bubbles(true);
    commit_init.set_cancelable(true);
    let commit_event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &commit_init)
            .expect("construct the Enter keydown event");
    let area_target: web_sys::EventTarget = area.clone().unchecked_into();
    area_target.dispatch_event(&commit_event).unwrap();
    next_tick().await;

    // An ACCEPTED commit closes the overlay (a Rejected outcome would instead
    // keep it open with the diagnostics underlined) — the overlay's
    // disappearance is the honest signal the commit ran and was accepted.
    assert!(
        query(&host, "[data-testid=\"sheet-cell-editor\"]").is_none(),
        "the overlay closes after an accepted commit"
    );

    // The canvas remains mounted and the debug readout still reports a real
    // drawn plan after the workbook dispatcher's re-projection repaint — no
    // blank state after the commit.
    assert!(
        query(&host, "[data-testid=\"sheet-canvas\"]").is_some(),
        "the canvas remains mounted after the commit's re-projection"
    );
    let plan = query(&host, "[data-testid=\"sheet-render-plan\"]")
        .expect("the debug readout still mounts after the commit");
    let cell_count: usize = plan
        .get_attribute("data-cell-count")
        .expect("data-cell-count is set")
        .parse()
        .expect("data-cell-count is a plain integer");
    assert!(
        cell_count > 0,
        "the plan still reports real cells after the commit's repaint, got {cell_count}"
    );
}

/// The element that currently owns keyboard focus — where the NEXT keystroke
/// lands (what the desktop shell's injected input follows).
fn active_element() -> web_sys::Element {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .active_element()
        .expect("something is focused")
}

fn is_focused(element: &web_sys::Node) -> bool {
    active_element().is_same_node(Some(element))
}

fn same_node(a: &web_sys::Node, b: &web_sys::Node) -> bool {
    a.is_same_node(Some(b))
}

/// The overlay editor's OWN degrade textarea (scoped to the overlay element, so
/// it can never be confused with the app's bridge-slot editor).
fn overlay_textarea(editor: &web_sys::Element) -> web_sys::HtmlTextAreaElement {
    editor
        .query_selector(".dna-bridge--degrade .dna-bridge__input")
        .unwrap()
        .expect("the overlay carries the degrade editor's textarea")
        .unchecked_into()
}

/// Focus the sheet section as a real pointer select does (the stage's
/// pointerdown handler calls `section.focus()`), so the keys that follow start
/// from the desktop click-through's exact state: focus on the grid.
fn focus_sheet_section(host: &web_sys::HtmlElement) -> web_sys::HtmlElement {
    let root: web_sys::HtmlElement = query(host, "[data-testid=\"sheet-root\"]")
        .expect("the sheet root mounts")
        .unchecked_into();
    root.focus().unwrap();
    root
}

/// dtc-j7n8.26 — the bead's repro, with every key dispatched at whatever element
/// actually holds focus (the way the desktop shell's injected input reaches the
/// page): focus on the grid section, F2 -> the overlay editor mounts seeded with
/// A1's authored `1` AND its textarea owns keyboard focus at once (before the
/// fix the section kept focus, so every further key re-ran the SELECT grammar);
/// the buffer takes `10`; Enter at the focused element COMMITS and closes the
/// overlay (before: Move(Down) on the section, overlay left open); focus returns
/// to the section so ArrowUp then F2 re-open A1 seeded with the committed `10`;
/// Esc reverts and hands focus back too. And the mast dirty dot is not just in
/// the DOM but PAINTED — it resolves `var(--dna-amber)`, which Strand never
/// emitted before, so the click-through saw no marker.
#[wasm_bindgen_test]
async fn calc_sheet_f2_hands_focus_to_the_editor_and_enter_commits_back_to_the_grid() {
    let host = mount();
    next_tick().await;
    next_tick().await;
    next_tick().await;
    let root = focus_sheet_section(&host);
    let root_target: web_sys::EventTarget = root.clone().unchecked_into();
    assert!(
        is_focused(&root),
        "precondition: the grid section holds focus, as after a pointer select"
    );

    // F2 on the section (nothing selected yet -> the origin, A1 = literal 1).
    press_chord(&root_target, "F2", false, false, false);
    next_tick().await;
    next_tick().await;
    let editor = query(&host, "[data-testid=\"sheet-cell-editor\"]")
        .expect("F2 opens the overlay editor at A1");
    let area = overlay_textarea(&editor);
    assert_eq!(area.value(), "1", "F2 seeds A1's authored literal");
    assert!(
        is_focused(&area),
        "the overlay textarea owns keyboard focus as soon as the editor mounts \
         (was: focus stayed on the section, so the next keys re-ran the SELECT grammar)"
    );

    // Replace the buffer with 10 (the browser's own text insertion, which a
    // synthetic keydown cannot trigger, is the input event) and press Enter at
    // the FOCUSED element — where a real keystroke lands.
    area.set_value("10");
    area.dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
    let focused: web_sys::EventTarget = active_element().unchecked_into();
    press_chord(&focused, "Enter", false, false, false);
    next_tick().await;
    next_tick().await;
    assert!(
        query(&host, "[data-testid=\"sheet-cell-editor\"]").is_none(),
        "Enter at the focused element COMMITS and closes the overlay \
         (was: Move(Down) on the section with the overlay left open)"
    );
    assert!(
        is_focused(&root),
        "an accepted commit hands focus back to the grid section"
    );

    // The mast dirty dot is in the DOM AND painted: a computed background that
    // is not transparent proves `var(--dna-amber)` resolved to a Strand token.
    let dot =
        query(&host, ".dna-mast__dirty-dot").expect("an accepted edit lights the mast dirty dot");
    let background = web_sys::window()
        .unwrap()
        .get_computed_style(&dot)
        .unwrap()
        .expect("computed style")
        .get_property_value("background-color")
        .unwrap();
    assert!(
        !matches!(background.as_str(), "" | "transparent" | "rgba(0, 0, 0, 0)"),
        "the dirty dot must be painted, got background-color {background:?}"
    );

    // Focus is back in the grammar: ArrowUp (the commit advanced to A2) then F2
    // re-open A1, now seeded with the committed 10.
    let focused: web_sys::EventTarget = active_element().unchecked_into();
    press_chord(&focused, "ArrowUp", false, false, false);
    next_tick().await;
    let focused: web_sys::EventTarget = active_element().unchecked_into();
    press_chord(&focused, "F2", false, false, false);
    next_tick().await;
    next_tick().await;
    let editor = query(&host, "[data-testid=\"sheet-cell-editor\"]")
        .expect("F2 from the section re-opens the editor");
    assert_eq!(
        editor.get_attribute("data-cell").as_deref(),
        Some("1:1"),
        "ArrowUp from A2 landed on A1 (the arrow reached the SELECT grammar)"
    );
    assert_eq!(
        overlay_textarea(&editor).value(),
        "10",
        "A1 now holds the committed 10"
    );

    // Esc at the focused element reverts and hands focus back as well.
    let focused: web_sys::EventTarget = active_element().unchecked_into();
    press_chord(&focused, "Escape", false, false, false);
    next_tick().await;
    next_tick().await;
    assert!(
        query(&host, "[data-testid=\"sheet-cell-editor\"]").is_none(),
        "Escape at the focused textarea reverts and closes the overlay"
    );
    assert!(
        is_focused(&root),
        "a revert hands focus back to the grid section"
    );
}

/// dtc-j7n8.26 — type-to-replace: a printable key on the grid section opens the
/// editor seeded with THAT character and hands it focus, so the SECOND keystroke
/// — dispatched at whatever element is focused, exactly as the desktop's injected
/// input reaches the page — lands in the editor instead of re-running the SELECT
/// grammar (before the fix `1` then `0` left a re-seeded editor reading `0`;
/// typing `xyz` left `z`).
#[wasm_bindgen_test]
async fn calc_sheet_type_to_replace_keeps_its_seed_under_the_next_keystroke() {
    let host = mount();
    next_tick().await;
    next_tick().await;
    next_tick().await;
    let root = focus_sheet_section(&host);
    let root_target: web_sys::EventTarget = root.clone().unchecked_into();

    press_chord(&root_target, "1", false, false, false);
    next_tick().await;
    next_tick().await;
    let editor = query(&host, "[data-testid=\"sheet-cell-editor\"]")
        .expect("a printable key opens the overlay editor (type-to-replace)");
    let area = overlay_textarea(&editor);
    assert_eq!(
        area.value(),
        "1",
        "the editor seeds with the typed character"
    );
    assert!(
        is_focused(&area),
        "the seeded editor's textarea owns keyboard focus at once"
    );

    // The second key lands wherever focus is.
    let focused: web_sys::EventTarget = active_element().unchecked_into();
    press_chord(&focused, "0", false, false, false);
    next_tick().await;
    next_tick().await;
    let editor_after = query(&host, "[data-testid=\"sheet-cell-editor\"]")
        .expect("the editor stays open under the second keystroke");
    assert!(
        same_node(&editor_after, &editor),
        "the second keystroke does NOT remount the editor (a re-seed would)"
    );
    assert_eq!(
        overlay_textarea(&editor_after).value(),
        "1",
        "the seed character survives the second keystroke (was: re-seeded to `0`)"
    );
    assert!(is_focused(&area), "focus stays in the editor");

    // The browser's own insertion appends (the input event): the buffer reads
    // 10 and Enter at the focused element commits it.
    area.set_value("10");
    area.dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
    let focused: web_sys::EventTarget = active_element().unchecked_into();
    press_chord(&focused, "Enter", false, false, false);
    next_tick().await;
    next_tick().await;
    assert!(
        query(&host, "[data-testid=\"sheet-cell-editor\"]").is_none(),
        "Enter commits the appended buffer and closes the overlay"
    );
    assert!(is_focused(&root), "focus returns to the grid section");
}

/// S3 follow-up (resize-reflow, dtc-4erh): the Canvas2D redraw effect reacts to
/// workspace / scroll / zoom / selection — but a window/container RESIZE is none
/// of those, so a `ResizeObserver` on the canvas re-fires the effect to
/// re-measure + resize the device-px backing store at the new size (otherwise the
/// fixed backing store would be CSS-scaled — stretched/blurry — until the next
/// interaction). This proves the observer path end-to-end: shrinking the mount
/// host reflows the canvas (CSS `width:100%`) narrower, and its OWN backing-store
/// width tracks the change.
#[wasm_bindgen_test]
async fn calc_sheet_canvas_repaints_when_it_resizes() {
    let host = mount();
    // A few ticks for the initial layout measure + draw (and the ResizeObserver's
    // own on-observe fire) to settle.
    next_tick().await;
    next_tick().await;
    next_tick().await;

    let canvas: web_sys::HtmlCanvasElement = query(&host, "[data-testid=\"sheet-canvas\"]")
        .expect("the sheet canvas mounts")
        .unchecked_into();
    let before = canvas.width();
    assert!(
        before > 0,
        "the canvas has a real device-px backing store initially, got {before}"
    );

    // Shrink the mount host, forcing the canvas (CSS `width:100%`) to reflow
    // narrower. The ResizeObserver fires and re-runs the redraw effect, which
    // re-measures `client_width` and resizes the backing store.
    host.style()
        .set_property("width", "420px")
        .expect("shrink the mount host");

    // ResizeObserver notifications are async (after layout); poll a bounded number
    // of ticks for the backing-store width to change — never a fixed sleep.
    let mut after = before;
    for _ in 0..30 {
        next_tick().await;
        after = canvas.width();
        if after != before {
            break;
        }
    }
    assert_ne!(
        after, before,
        "resizing re-measures the canvas backing store (resize-reflow): {before} -> {after}"
    );
    assert!(
        after > 0,
        "the resized canvas still has a real backing store, got {after}"
    );
}

/// S3 follow-up (nav auto-scroll, dtc-m20s): arrow-key nav moves the active cell,
/// but a step PAST the visible window must scroll the viewport so the cell stays
/// on screen (otherwise the highlight draws off-screen and is clipped away). This
/// drives ArrowDown well past the canvas's visible rows, then opens the editor on
/// the deep active cell and proves — via the editor overlay's real on-screen
/// position (it is placed at `cell_rect(metrics, scrolled_viewport, row, col)`) —
/// that the cell was revealed near the viewport's bottom edge rather than left at
/// its far-below unscrolled position.
#[wasm_bindgen_test]
async fn calc_sheet_scrolls_the_active_cell_into_view_on_keyboard_nav() {
    let host = mount();
    // Settle the initial layout measure + first paint (which feeds the measured
    // canvas size back into the shared viewport that the reveal math reads).
    next_tick().await;
    next_tick().await;
    next_tick().await;

    let canvas: web_sys::HtmlCanvasElement = query(&host, "[data-testid=\"sheet-canvas\"]")
        .expect("the sheet canvas mounts")
        .unchecked_into();
    let canvas_h = f64::from(canvas.client_height());
    assert!(
        canvas_h > 0.0,
        "the canvas has a real measured height for the reveal math, got {canvas_h}"
    );

    // Default metrics at zoom 1.0: 22px rows + a 22px column-header strip
    // (`GridMetrics::default`). Rows that fit in the data area, then aim ~20 rows
    // PAST that so the target is unambiguously below the visible window.
    let row_h = 22.0_f64;
    let header_h = 22.0_f64;
    let rows_visible = ((canvas_h - header_h) / row_h).floor().max(1.0) as u32;
    let target_row = rows_visible + 20;

    // The unscrolled viewport-y of the target cell — where its editor WOULD sit
    // with no auto-scroll (far below the canvas). The test is only meaningful if
    // that position is genuinely off-screen.
    let unscrolled_top = header_h + (f64::from(target_row) - 1.0) * row_h;
    assert!(
        unscrolled_top > canvas_h,
        "test set-up must put the target row off-screen: unscrolled_top {unscrolled_top} > canvas_h {canvas_h}"
    );

    // Focus the sheet section and step down `target_row` times. The first
    // ArrowDown (nothing selected) lands on A1, each subsequent one moves down one
    // row, so `target_row` presses reach row `target_row`. Each press reveals the
    // moved cell synchronously (the reveal reads the viewport untracked and
    // accumulates scroll), so no per-press tick is needed.
    let root = query(&host, "[data-testid=\"sheet-root\"]").expect("the sheet root mounts");
    let root_target: web_sys::EventTarget = root.clone().unchecked_into();
    for _ in 0..target_row {
        let init = web_sys::KeyboardEventInit::new();
        init.set_key("ArrowDown");
        init.set_bubbles(true);
        init.set_cancelable(true);
        let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
            .expect("construct the ArrowDown keydown");
        root_target.dispatch_event(&event).unwrap();
    }

    // F2 opens the ONE overlay editor at the (now deep) active cell.
    let f2_init = web_sys::KeyboardEventInit::new();
    f2_init.set_key("F2");
    f2_init.set_bubbles(true);
    f2_init.set_cancelable(true);
    let f2_event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &f2_init)
        .expect("construct the F2 keydown");
    root_target.dispatch_event(&f2_event).unwrap();
    next_tick().await;

    let editor = query(&host, "[data-testid=\"sheet-cell-editor\"]")
        .expect("F2 opens the overlay editor at the deep active cell");
    assert_eq!(
        editor.get_attribute("data-cell").as_deref(),
        Some(format!("{target_row}:1").as_str()),
        "the active cell walked down to row {target_row} (column 1)"
    );

    // The editor sits at the active cell's real on-screen rect. Its offset from the
    // canvas top IS the cell's scrolled viewport-y: with auto-scroll it is near the
    // bottom of the visible canvas; without it, it would be at `unscrolled_top`,
    // far below. Measured via bounding rects (no inline-style string parsing).
    let canvas_top = canvas.get_bounding_client_rect().top();
    let editor_top = editor.get_bounding_client_rect().top() - canvas_top;
    assert!(
        editor_top < unscrolled_top,
        "auto-scroll revealed the cell (editor top {editor_top} must be above its unscrolled {unscrolled_top})"
    );
    assert!(
        editor_top >= 0.0 && editor_top <= canvas_h,
        "the revealed cell sits within the visible canvas [0, {canvas_h}], got {editor_top}"
    );
}

/// S3 follow-up (Tab-in-editor commits and moves RIGHT, dtc-dzky): Enter-commit
/// advances DOWN, but a Tab-commit must advance RIGHT (Excel grid entry). The
/// Sheet opts its overlay editor into the bridge's `commit_on_tab`, so Tab emits
/// `CommitRequested { advance: Right }` and the Sheet commits + moves the active
/// cell one COLUMN right. Proven end-to-end: open A1, type, Tab, then reopen the
/// editor and confirm it lands on B1 (row 1, col 2) — RIGHT, not the Enter-path
/// 2:1. (The demo workbook is cells-only, so every cell is editable and exposes
/// `data-cell`.)
#[wasm_bindgen_test]
async fn calc_sheet_tab_in_editor_commits_and_moves_right() {
    let host = mount();
    next_tick().await;
    next_tick().await;
    next_tick().await;

    let root = query(&host, "[data-testid=\"sheet-root\"]").expect("the sheet root mounts");
    let root_target: web_sys::EventTarget = root.clone().unchecked_into();
    let press_f2 = || {
        let init = web_sys::KeyboardEventInit::new();
        init.set_key("F2");
        init.set_bubbles(true);
        init.set_cancelable(true);
        let ev = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
            .expect("construct the F2 keydown");
        root_target.dispatch_event(&ev).unwrap();
    };

    // F2 opens the overlay editor at A1 (nothing selected → the origin).
    press_f2();
    next_tick().await;
    let editor =
        query(&host, "[data-testid=\"sheet-cell-editor\"]").expect("F2 opens the overlay at A1");
    assert_eq!(
        editor.get_attribute("data-cell").as_deref(),
        Some("1:1"),
        "the editor opens at A1"
    );

    // Type a literal, then commit with TAB (not Enter).
    let area = editor
        .query_selector(".dna-bridge--degrade .dna-bridge__input")
        .unwrap()
        .expect("the degrade editor mounts")
        .unchecked_into::<web_sys::HtmlTextAreaElement>();
    area.set_value("77");
    area.dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
    let tab_init = web_sys::KeyboardEventInit::new();
    tab_init.set_key("Tab");
    tab_init.set_bubbles(true);
    tab_init.set_cancelable(true);
    let tab_event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &tab_init)
        .expect("construct the Tab keydown");
    let area_target: web_sys::EventTarget = area.clone().unchecked_into();
    area_target.dispatch_event(&tab_event).unwrap();
    next_tick().await;

    // An accepted commit closed the overlay (only a Rejected outcome keeps it open).
    assert!(
        query(&host, "[data-testid=\"sheet-cell-editor\"]").is_none(),
        "an accepted Tab-commit closes the overlay"
    );

    // Reopen the editor on the post-Tab active cell: it walked one column RIGHT to
    // B1 (row 1, col 2) — NOT the Enter-path 2:1.
    press_f2();
    next_tick().await;
    let editor2 = query(&host, "[data-testid=\"sheet-cell-editor\"]")
        .expect("F2 reopens the overlay at the post-Tab active cell");
    assert_eq!(
        editor2.get_attribute("data-cell").as_deref(),
        Some("1:2"),
        "Tab-commit advanced the active cell one column RIGHT (B1), not down to 2:1"
    );
}

/// Like [`press_chord`] but reports whether a handler called `preventDefault`
/// — the shell's Save / Open arms do (they intercept the browser default and
/// forward the verb), so a prevented Ctrl+S is the event-level signature of
/// the `.dna-shell` keydown pipeline having claimed the chord.
fn press_chord_observed(
    target: &web_sys::EventTarget,
    key: &str,
    ctrl: bool,
    alt: bool,
    shift: bool,
) -> bool {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_ctrl_key(ctrl);
    init.set_alt_key(alt);
    init.set_shift_key(shift);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    target.dispatch_event(&event).unwrap();
    event.default_prevented()
}

/// The document line's lifecycle readout: `(data-document-status, detail text)`.
fn document_line_status(host: &web_sys::HtmlElement) -> (String, String) {
    let line = query(host, "[data-testid=\"calc-document\"]").expect("the document line mounts");
    let status = line
        .get_attribute("data-document-status")
        .expect("data-document-status is set");
    let detail = line
        .query_selector(".calc-document__detail")
        .unwrap()
        .expect("the document line carries its detail span")
        .text_content()
        .unwrap_or_default();
    (status, detail)
}

/// One Open -> Save round from `target` (dtc-j7n8.25): Ctrl+O first (a fresh
/// `open` status, and Ctrl+O's own routing stays proven), then Ctrl+S must
/// flip the document line to the `save` note. Each read waits a tick so the
/// document line's reactive attributes have rendered the controller's status.
/// Returns Ctrl+S's `defaultPrevented`.
async fn open_then_save(
    host: &web_sys::HtmlElement,
    target: &web_sys::EventTarget,
    state: &str,
) -> bool {
    let open_prevented = press_chord_observed(target, "o", true, false, false);
    next_tick().await;
    let (status, detail) = document_line_status(host);
    assert!(
        open_prevented,
        "[{state}] Ctrl+O must reach the shell's Open arm (defaultPrevented)"
    );
    assert_eq!(status, "unavailable", "[{state}] Ctrl+O ran the Open verb");
    assert!(
        detail.to_lowercase().contains("open"),
        "[{state}] the document line names the Open verb, got {detail:?}"
    );
    let save_prevented = press_chord_observed(target, "s", true, false, false);
    next_tick().await;
    let (status, detail) = document_line_status(host);
    assert_eq!(
        status, "unavailable",
        "[{state}] Ctrl+S must run the shell's Save verb (run_file_verb -> note_bridge_unavailable), got status {status:?} / {detail:?}"
    );
    assert!(
        detail.to_lowercase().contains("save"),
        "[{state}] the document line must name the Save verb after Ctrl+S (the desktop saw no document-line change), got {detail:?}"
    );
    save_prevented
}

/// dtc-j7n8.25 — the bead's acceptance test: the shell's Save verb fires for
/// Ctrl+S with the sheet SECTION focused, end to end through the real composed
/// app (`SheetStage` section keydown -> `.dna-shell` `handle_keydown` ->
/// `on_shell_verb` -> `run_file_verb`). The browser runtime has no desktop
/// file bridge, so a verb that reaches `run_file_verb` is answered by
/// `note_bridge_unavailable` (document.rs): the document line flips to
/// `data-document-status="unavailable"` with the VERB's label in the detail —
/// the observable the desktop click-through never saw change. Ctrl+O is
/// pressed before every Ctrl+S so each Save assertion is a fresh `open` ->
/// `save` transition (and Ctrl+O's own routing stays proven — it worked on the
/// desktop and must keep working). `defaultPrevented` proves the shell's Save
/// arm (not some other handler) claimed the chord.
///
/// Three focus states, each dispatched where a real keystroke lands:
///   1. SELECT mode after a pointer click on a cell (the section holds focus)
///      — the literal state observed on the desktop 2026-09-02;
///   2. EDITING after F2, the chord at the overlay editor's textarea (the
///      degrade editor used to `stop_propagation()` every keydown);
///   3. EDITING with the chord at the SECTION (focus fell out of the editor:
///      the `ShellOwns` route — the refocus arm used to consume it), and the
///      editor stays open with its buffer untouched.
#[wasm_bindgen_test]
async fn calc_sheet_ctrl_s_fires_the_shell_save_verb_from_every_stage_focus() {
    let host = mount();
    next_tick().await;
    next_tick().await;
    next_tick().await;

    // Precondition: no bridge in this runtime, and no lifecycle verb has run.
    let line = query(&host, "[data-testid=\"calc-document\"]").expect("the document line mounts");
    assert_eq!(
        line.get_attribute("data-file-bridge").as_deref(),
        Some("unavailable"),
        "the browser test runtime has no desktop file bridge"
    );
    assert_eq!(
        document_line_status(&host).0,
        "none",
        "no lifecycle verb has run before any key"
    );

    // 1. Click B2 on the canvas (`pointerdown`, the stage's real selection
    //    handler): SELECT mode, the section holds focus. B2 = row 2, col 2 sits
    //    at the default metrics' `cell_rect(2, 2)`: center `(48 + 80 + 40,
    //    22 + 22 + 11) = (168, 55)` in the canvas's own space.
    let canvas = query(&host, "[data-testid=\"sheet-canvas\"]").expect("the canvas mounts");
    let rect = canvas.get_bounding_client_rect();
    let click = web_sys::MouseEventInit::new();
    click.set_client_x((rect.x() + 168.0).round() as i32);
    click.set_client_y((rect.y() + 55.0).round() as i32);
    click.set_bubbles(true);
    click.set_cancelable(true);
    let pointerdown = web_sys::MouseEvent::new_with_mouse_event_init_dict("pointerdown", &click)
        .expect("construct the pointerdown event");
    let canvas_target: web_sys::EventTarget = canvas.clone().unchecked_into();
    canvas_target.dispatch_event(&pointerdown).unwrap();
    next_tick().await;
    let root: web_sys::HtmlElement = query(&host, "[data-testid=\"sheet-root\"]")
        .expect("the sheet root mounts")
        .unchecked_into();
    assert!(
        is_focused(&root),
        "a pointer select hands focus to the sheet section (the desktop's state)"
    );
    assert!(
        query(&host, "[data-testid=\"sheet-cell-editor\"]").is_none(),
        "a single click SELECTS; no editor is open"
    );
    let focused: web_sys::EventTarget = active_element().unchecked_into();
    assert!(
        open_then_save(&host, &focused, "SELECT, section focused").await,
        "[SELECT, section focused] Ctrl+S must be claimed by the shell's Save arm \
         (defaultPrevented), not left to the browser"
    );
    next_tick().await;
    assert!(
        query(&host, "[data-testid=\"sheet-cell-editor\"]").is_none(),
        "Ctrl+S / Ctrl+O in SELECT mode open no editor"
    );

    // 2. F2 at the focused section opens the overlay at the CLICKED cell (B2 —
    //    proving the pointerdown selected it) and hands its textarea focus; the
    //    chord is dispatched at that textarea.
    let focused: web_sys::EventTarget = active_element().unchecked_into();
    press_chord(&focused, "F2", false, false, false);
    next_tick().await;
    next_tick().await;
    let editor = query(&host, "[data-testid=\"sheet-cell-editor\"]")
        .expect("F2 opens the overlay editor at the selected cell");
    assert_eq!(
        editor.get_attribute("data-cell").as_deref(),
        Some("2:2"),
        "the pointerdown selected B2 and F2 opened the editor there"
    );
    let area = overlay_textarea(&editor);
    // B2's authored text on the active sheet (the demo's `=A2*10`), captured
    // rather than assumed so the final buffer check compares against host truth.
    let seed = area.value();
    assert!(
        !seed.is_empty(),
        "F2 seeds the editor with B2's authored text"
    );
    assert!(
        is_focused(&area),
        "the overlay textarea owns focus while EDITING (dtc-j7n8.26)"
    );
    let focused: web_sys::EventTarget = active_element().unchecked_into();
    assert!(
        open_then_save(&host, &focused, "EDITING, textarea focused").await,
        "[EDITING, textarea focused] Ctrl+S must be claimed by the shell's Save arm"
    );
    next_tick().await;
    assert!(
        query(&host, "[data-testid=\"sheet-cell-editor\"]").is_some(),
        "a shell chord from the textarea neither commits nor reverts the editor"
    );

    // 3. Still EDITING, the chord at the SECTION (focus fell out of the editor):
    //    the `ShellOwns` route — the section must leave it untouched so it
    //    bubbles to the shell, and the editor stays open.
    let root_target: web_sys::EventTarget = root.clone().unchecked_into();
    assert!(
        open_then_save(&host, &root_target, "EDITING, chord at the section").await,
        "[EDITING, chord at the section] Ctrl+S must be claimed by the shell's Save arm"
    );
    next_tick().await;
    let editor = query(&host, "[data-testid=\"sheet-cell-editor\"]")
        .expect("a shell chord at the section while EDITING leaves the editor open");
    assert_eq!(
        overlay_textarea(&editor).value(),
        seed,
        "the editor's buffer is untouched by the shell chords"
    );
}

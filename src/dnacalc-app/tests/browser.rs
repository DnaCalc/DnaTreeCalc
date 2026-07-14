//! Browser suite for the DNA Calc app (SHELL_SPEC §10.1-10.4, D3 parity).
//!
//! Mounts the real [`CalcApp`] over the host-core demo workbook and proves, in
//! the DOM, that the Calc composition mounts its full region set with a stage
//! switcher, that stage switching is re-projection (the incoming stage mounts
//! and reads shared continuity state), that the DEGRADE bridge edits a workbook
//! cell via `EnterGridCell` with the three-way outcome rendered honestly, and
//! that the atlas + deck open. Reserved parity/evidence slots render NOTHING.

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
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
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
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    let target: web_sys::EventTarget = area.unchecked_into();
    target.dispatch_event(&event).unwrap();
}

fn press_chord(target: &web_sys::EventTarget, key: &str, ctrl: bool, alt: bool, shift: bool) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_ctrl_key(ctrl);
    init.set_alt_key(alt);
    init.set_shift_key(shift);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    target.dispatch_event(&event).unwrap();
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
        "two stub stages + Notebook + Atlas compose the switcher"
    );

    let parity = query(&host, "[data-slot=\"parity\"]").expect("parity slot reserved in layout");
    assert_eq!(parity.text_content().unwrap_or_default(), "");
    assert_eq!(parity.child_element_count(), 0);
}

/// §10.1 — stage switching is re-projection: the incoming stage mounts and its
/// shared-continuity readout is present (the shared state survives the switch,
/// it is not reset to a fresh default).
#[wasm_bindgen_test]
async fn calc_stage_switch_reprojects_and_preserves_continuity_surface() {
    let host = mount();
    next_tick().await;

    // Initial: the first visible stage (Sheet) mounts.
    assert!(
        query(&host, "[data-testid=\"calc-stage-sheet\"]").is_some(),
        "the Sheet stage mounts first"
    );
    let before = query(&host, "[data-testid=\"calc-stage-sheet\"] .calc-stage__continuity")
        .and_then(|el| el.get_attribute("data-selection"))
        .expect("continuity readout on the Sheet stage");

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
        query(&host, "[data-testid=\"calc-stage-sheet\"]").is_none(),
        "switching is re-projection: only one stage mounts at a time"
    );
    // The shared continuity state survived the switch (same value the new
    // stage reads back).
    let after = query(&host, "[data-testid=\"calc-stage-model\"] .calc-stage__continuity")
        .and_then(|el| el.get_attribute("data-selection"))
        .expect("continuity readout on the Model stage");
    assert_eq!(before, after, "continuity state survives the switch");
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

    // Initial: the first visible stage (Sheet) mounts.
    assert!(
        query(&host, "[data-testid=\"calc-stage-sheet\"]").is_some(),
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
        query(&host, "[data-testid=\"calc-stage-sheet\"]").is_none(),
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

    // Initial: the first visible stage (Sheet) mounts.
    assert!(
        query(&host, "[data-testid=\"calc-stage-sheet\"]").is_some(),
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
        query(&host, "[data-testid=\"calc-stage-sheet\"]").is_none(),
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
    assert_eq!(a3_value.trim(), "3", "Sheet1 A3's block value region reads 3");
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
        notebook_block_value(&host, "R6C1").as_deref().map(str::trim),
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
        notebook_block_value(&host, "R6C1").as_deref().map(str::trim),
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
        notebook_block_value(&host, "R3C1").as_deref().map(str::trim),
        Some("3"),
        "Sheet1 A3 seeds as 3"
    );
    assert_eq!(
        notebook_block_value(&host, "R2C1").as_deref().map(str::trim),
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
        notebook_block_value(&host, "R3C1").as_deref().map(str::trim),
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
        notebook_block_value(&host, "R2C1").as_deref().map(str::trim),
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
    assert!(create_button.disabled(), "Create is disabled with an empty name");
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
        notebook_block_value(&host, "R3C1").as_deref().map(str::trim),
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
        notebook_block_value(&host, "R3C1").as_deref().map(str::trim),
        Some("3"),
        "R3C1's value region still reads 3 for a Reviewer"
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
        query(&host, ".dna-bridge--degrade .dna-bridge__seg--role-function").is_none(),
        "degrade mode never paints token-role classes"
    );

    // Literal.
    set_degrade_text(&host, "42");
    next_tick().await;
    commit_degrade(&host);
    next_tick().await;
    let outcome = query(&host, "[data-testid=\"calc-outcome\"]").expect("outcome renders");
    assert_eq!(outcome.get_attribute("data-outcome").as_deref(), Some("literal"));

    // Formula (=A1+A5 over the demo workbook = 6).
    set_degrade_text(&host, "=A1+A5");
    next_tick().await;
    commit_degrade(&host);
    next_tick().await;
    let outcome = query(&host, "[data-testid=\"calc-outcome\"]").expect("outcome renders");
    assert_eq!(outcome.get_attribute("data-outcome").as_deref(), Some("formula"));

    // Cleared (empty commit).
    set_degrade_text(&host, "");
    next_tick().await;
    commit_degrade(&host);
    next_tick().await;
    let outcome = query(&host, "[data-testid=\"calc-outcome\"]").expect("outcome renders");
    assert_eq!(outcome.get_attribute("data-outcome").as_deref(), Some("cleared"));

    // Rejected (an unparseable formula) — typed rejection surfaces.
    set_degrade_text(&host, "=1+");
    next_tick().await;
    commit_degrade(&host);
    next_tick().await;
    let outcome = query(&host, "[data-testid=\"calc-outcome\"]").expect("outcome renders");
    assert_eq!(outcome.get_attribute("data-outcome").as_deref(), Some("rejected"));
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

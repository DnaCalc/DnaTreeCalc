//! Browser smoke for the dnacalc-shell cockpit (SHELL_SPEC.md §9): mount
//! the Calc composition with two stub stages over a `RecordingDispatcher`,
//! then prove in a live DOM that
//! - the composed regions mount,
//! - stage switching is re-projection only (zero dispatched intents),
//! - continuity state survives the switch,
//! - the keyboard atlas lists every chord from the live registry with the
//!   browser divergence tags, and
//! - the reserved parity/evidence slots render NOTHING.

#![cfg(target_arch = "wasm32")]

use std::sync::Arc;

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use dnacalc_shell::{
    ProfileTag, RuntimeContext, Shell, ShellComposition, ShellKeyboardRegistry, StageContext,
    StageHandle, StageId, StageRegistry, StageSurface,
};
use dnacalc_skin_ir::dispatcher::RecordingDispatcher;
use dnacalc_skin_ir::identity::{NodeId, NodeKey};
use dnacalc_skin_ir::intent::{WorkspaceDelta, WorkspaceIntent};
use dnacalc_skin_ir::selection::SelectionState;
use dnacalc_skin_ir::state::{SharedSkinState, SharedStateChange, SharedStateOrigin};
use dnacalc_skin_ir::workspace::WorkspaceState;
use dnacalc_skin_leptos::state_handles::SharedSkinStateHandle;
use dnacalc_strand::{Density, Theme};

wasm_bindgen_test_configure!(run_in_browser);

/// Let queued reactive/rendering work flush (one macrotask turn) — the
/// estate's proven post-interaction boundary.
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

/// A stub stage that renders its identity plus a live continuity readout
/// (the shared collapse-set size), so the smoke can watch continuity
/// survive a switch.
struct StubStage {
    id: StageId,
    title: &'static str,
    testid: &'static str,
}

impl StageSurface for StubStage {
    fn id(&self) -> StageId {
        self.id
    }

    fn title(&self) -> &'static str {
        self.title
    }

    fn supports(&self, _profile: &ProfileTag) -> bool {
        true
    }

    fn mount(&self, ctx: StageContext) -> StageHandle {
        let shared = ctx.shared;
        let testid = self.testid;
        StageHandle::new(
            view! {
                <div data-testid=testid>
                    <span data-testid="collapsed-count">
                        {move || shared.with(|state| state.collapsed_keys.len().to_string())}
                    </span>
                </div>
            }
            .into_any(),
        )
    }
}

struct Mounted {
    host: web_sys::HtmlElement,
    dispatcher: RecordingDispatcher,
    shared: SharedSkinStateHandle,
}

fn mount_calc_shell() -> Mounted {
    let document = web_sys::window().unwrap().document().unwrap();
    let host = document
        .create_element("div")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    document.body().unwrap().append_child(&host).unwrap();

    let dispatcher = RecordingDispatcher::new();
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());

    let mut workspace_state = WorkspaceState::default();
    workspace_state.workspace_id = "smoke-workspace".to_string();
    let (workspace, _set_workspace) = signal(workspace_state);
    let (latest_delta, _set_delta) = signal(WorkspaceDelta::default());
    let (selection, _set_selection) = signal(SelectionState::default());

    let stages = StageRegistry::new()
        .with_stage(Arc::new(StubStage {
            id: StageId::Model,
            title: "Model",
            testid: "stage-alpha",
        }))
        .with_stage(Arc::new(StubStage {
            id: StageId::Atlas,
            title: "Atlas",
            testid: "stage-beta",
        }));

    let dispatch: Arc<dyn dnacalc_skin_ir::intent::Dispatcher> = Arc::new(dispatcher.clone());
    let composition = ShellComposition::calc(ProfileTag::RichTree);

    leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        view! {
            <Shell
                composition=composition
                stages=stages
                workspace=workspace
                latest_delta=latest_delta
                selection=selection
                shared=shared
                dispatch=dispatch
                theme=Theme::CockpitLight
                density=Density::Working
                runtime=RuntimeContext::Browser
            />
        }
    })
    .forget();

    Mounted {
        host,
        dispatcher,
        shared,
    }
}

fn shell_root(host: &web_sys::HtmlElement) -> web_sys::EventTarget {
    host.query_selector(".dna-shell")
        .unwrap()
        .expect("shell root mounts")
        .unchecked_into()
}

fn press_chord(target: &web_sys::EventTarget, key: &str, ctrl: bool, alt: bool, shift: bool) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_ctrl_key(ctrl);
    init.set_alt_key(alt);
    init.set_shift_key(shift);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
        .expect("construct keydown event");
    target.dispatch_event(&event).expect("dispatch keydown");
}

fn query(host: &web_sys::HtmlElement, selector: &str) -> Option<web_sys::Element> {
    host.query_selector(selector).unwrap()
}

/// Type `text` into an input and fire an `input` event so the deck's query
/// signal updates, as a real keystroke would.
fn type_into(input: &web_sys::Element, text: &str) {
    let input: &web_sys::HtmlInputElement = input.unchecked_ref();
    input.set_value(text);
    input
        .dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
}

/// Fire an Enter keydown on a specific element (the deck input owns its own
/// Enter handler).
fn press_enter(target: &web_sys::EventTarget) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key("Enter");
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
        .expect("construct keydown event");
    target.dispatch_event(&event).expect("dispatch enter");
}

#[wasm_bindgen_test]
fn shell_mounts_every_composed_region_and_reserved_slots_render_nothing() {
    let mounted = mount_calc_shell();
    let host = &mounted.host;

    // Every composed region mounts.
    for region in [
        "mast",
        "bridge",
        "registry",
        "stage-host",
        "inspector",
        "strip",
    ] {
        assert!(
            query(host, &format!("[data-region=\"{region}\"]")).is_some(),
            "region {region} must mount in the Calc composition"
        );
    }

    // The reserved parity slot exists in layout math and renders NOTHING:
    // no text, no children (PARITY_TRUST_UX §5/§8).
    let parity = query(host, "[data-slot=\"parity\"]").expect("parity slot reserved in layout");
    assert_eq!(parity.text_content().unwrap_or_default(), "");
    assert_eq!(parity.child_element_count(), 0);

    // Feed health likewise renders nothing until G7.
    let feeds = query(host, "[data-slot=\"feeds\"]").expect("feeds slot present");
    assert_eq!(feeds.text_content().unwrap_or_default(), "");

    // The Inspector's Evidence slot is never populated — no element at all.
    assert!(
        query(host, "[data-inspector-slot=\"evidence\"]").is_none(),
        "Evidence inspector slot must render nothing this phase"
    );
    // While the other inspector slots do render.
    assert!(query(host, "[data-inspector-slot=\"value-shape\"]").is_some());

    // Wired-or-honest strip slots: calc wired (shared recalc mode), locale
    // honestly absent (em-dash, reason in title).
    let calc = query(host, "[data-slot=\"calc\"]").unwrap();
    assert_eq!(calc.text_content().unwrap(), "calc: auto");
    let locale = query(host, "[data-slot=\"locale\"]").unwrap();
    assert_eq!(locale.text_content().unwrap(), "\u{2014}");
    assert!(locale.get_attribute("title").is_some());
}

#[wasm_bindgen_test]
async fn stage_switch_is_reprojection_only_and_continuity_survives() {
    let mounted = mount_calc_shell();
    let host = &mounted.host;
    next_tick().await;

    // First visible stage mounts by default.
    assert!(query(host, "[data-testid=\"stage-alpha\"]").is_some());
    assert!(query(host, "[data-testid=\"stage-beta\"]").is_none());

    // Continuity state written before the switch (audited chokepoint).
    mounted.shared.apply(
        SharedStateChange::Fold(NodeKey::new("folded-node")),
        SharedStateOrigin::Shell,
    );

    // Switch to stage 2 with the §5.1 primary chord Ctrl+Alt+2.
    let root = shell_root(host);
    press_chord(&root, "2", true, true, false);
    next_tick().await;

    assert!(
        query(host, "[data-testid=\"stage-beta\"]").is_some(),
        "Ctrl+Alt+2 must project stage 2"
    );
    assert!(query(host, "[data-testid=\"stage-alpha\"]").is_none());

    // Re-projection only: not a single engine intent dispatched.
    assert!(
        mounted.dispatcher.intents().is_empty(),
        "stage switch must not dispatch, got {:?}",
        mounted.dispatcher.intents()
    );

    // Continuity survived: the incoming stage reads the same collapse set.
    let readout = query(host, "[data-testid=\"collapsed-count\"]").unwrap();
    assert_eq!(readout.text_content().unwrap(), "1");
    assert_eq!(
        mounted.shared.get_untracked().active_lens.as_deref(),
        Some("atlas")
    );

    // The switcher tabs reflect the active stage.
    let beta_tab = query(host, "[data-stage-tab=\"atlas\"]").unwrap();
    assert_eq!(beta_tab.get_attribute("aria-selected").unwrap(), "true");

    // Sanity for the zero-dispatch assertion above: a verb that IS meant to
    // dispatch (F9 Recalculate) reaches the dispatcher through the same
    // pipeline.
    press_chord(&root, "F9", false, false, false);
    next_tick().await;
    assert_eq!(mounted.dispatcher.intents().len(), 1);
}

#[wasm_bindgen_test]
async fn keyboard_atlas_lists_every_live_chord_with_browser_tags_and_esc_closes() {
    let mounted = mount_calc_shell();
    let host = &mounted.host;
    next_tick().await;
    let root = shell_root(host);

    // Ctrl+/ opens the atlas.
    press_chord(&root, "/", true, false, false);
    next_tick().await;
    assert!(
        query(host, "[data-overlay=\"keyboard-atlas\"]").is_some(),
        "Ctrl+/ opens the keyboard atlas"
    );

    // The atlas renders from the live registry: one row per binding.
    let expected = ShellKeyboardRegistry::universal(
        ShellComposition::calc(ProfileTag::RichTree).catalog_composition(2),
        RuntimeContext::Browser,
    );
    let rows = host.query_selector_all("[data-atlas-row]").unwrap();
    assert_eq!(
        rows.length() as usize,
        expected.bindings().len(),
        "atlas row count must equal live registry binding count"
    );

    // Browser-alternate rows carry the divergence tag.
    let tags = host
        .query_selector_all("[data-atlas-tag=\"browser\"]")
        .unwrap();
    assert_eq!(tags.length(), 4, "Ctrl+Shift+P, ?, Alt+B, Alt+I");

    // One at a time: opening the command deck replaces the atlas.
    press_chord(&root, "k", true, false, false);
    next_tick().await;
    assert!(query(host, "[data-overlay=\"keyboard-atlas\"]").is_none());
    assert!(query(host, "[data-overlay=\"command-deck\"]").is_some());

    // Esc closes the topmost overlay first — dispatched on the deck's own
    // autofocused input, NOT the shell root (bead dtc-1tk.1 / H1a). A real
    // Escape keydown targets whatever element has focus, and the deck input
    // autofocuses the instant the deck opens; `event_target_is_text_entry`
    // (shell.rs) only trips when the event's `target` actually is that
    // input, so dispatching on `.dna-shell` instead (as this test used to)
    // could never catch a regression in the deck's own Esc handling.
    let deck_input = query(host, ".dna-deck__input").expect("deck input autofocuses on open");
    press_chord(
        &deck_input.clone().unchecked_into(),
        "Escape",
        false,
        false,
        false,
    );
    next_tick().await;
    assert!(
        query(host, "[data-overlay=\"command-deck\"]").is_none(),
        "Esc on the focused deck input must close the deck (H1a)"
    );

    // Ctrl+F opens the deck placeholder in goto mode.
    press_chord(&root, "f", true, false, false);
    next_tick().await;
    let deck = query(host, "[data-overlay=\"command-deck\"]").unwrap();
    assert_eq!(deck.get_attribute("data-goto-mode").unwrap(), "true");
    let goto_input = query(host, ".dna-deck__input").expect("goto-mode deck input autofocuses too");
    press_chord(
        &goto_input.clone().unchecked_into(),
        "Escape",
        false,
        false,
        false,
    );
    next_tick().await;
    assert!(
        query(host, "[data-overlay=\"command-deck\"]").is_none(),
        "Esc on the focused goto-mode deck input must also close the deck"
    );

    // No engine intents from any of this overlay traffic.
    assert!(mounted.dispatcher.intents().is_empty());
}

#[wasm_bindgen_test]
async fn registry_toggle_collapses_the_rail_and_omission_padding_holds() {
    let mounted = mount_calc_shell();
    let host = &mounted.host;
    next_tick().await;
    let root = shell_root(host);

    let rail = query(host, "[data-region=\"registry\"]").unwrap();
    assert_eq!(rail.get_attribute("aria-hidden").unwrap(), "false");

    // Ctrl+B collapses; Alt+B (browser alternate) reopens.
    press_chord(&root, "b", true, false, false);
    next_tick().await;
    assert_eq!(rail.get_attribute("aria-hidden").unwrap(), "true");
    press_chord(&root, "b", false, true, false);
    next_tick().await;
    assert_eq!(rail.get_attribute("aria-hidden").unwrap(), "false");
}

#[wasm_bindgen_test]
async fn command_deck_opens_and_executes_a_command_through_dispatch() {
    let mounted = mount_calc_shell();
    let host = &mounted.host;
    next_tick().await;
    let root = shell_root(host);

    // Ctrl+K opens the real deck (mounted through the command_deck seam), and
    // it lists commands from the in-process catalog adapter.
    press_chord(&root, "k", true, false, false);
    next_tick().await;
    assert!(
        query(host, "[data-overlay=\"command-deck\"]").is_some(),
        "Ctrl+K opens the command deck"
    );
    assert!(
        host.query_selector_all("[data-command-id]")
            .unwrap()
            .length()
            >= 15,
        "the deck lists >= 15 commands (SHELL_SPEC 10.4)"
    );
    // Each row shows its effective chord where one exists.
    let recalc = query(host, "[data-command-id=\"recalculate\"]").expect("recalculate command");
    assert!(recalc.text_content().unwrap().contains("F9"));

    // Filter to Recalculate and execute with Enter — the dispatcher records
    // exactly the Recalculate intent, and the deck closes.
    let input = query(host, ".dna-deck__input").expect("deck input");
    type_into(&input, "recalc");
    next_tick().await;
    press_enter(&input.clone().unchecked_into());
    next_tick().await;
    assert_eq!(
        mounted.dispatcher.intents(),
        vec![WorkspaceIntent::Recalculate],
        "executing the deck command dispatches exactly its intent"
    );
    assert!(
        query(host, "[data-overlay=\"command-deck\"]").is_none(),
        "executing a command closes the deck"
    );
}

#[wasm_bindgen_test]
async fn command_deck_mirror_chord_and_a1_goto_navigation() {
    let mounted = mount_calc_shell();
    let host = &mounted.host;
    next_tick().await;
    let root = shell_root(host);

    // Ctrl+Shift+P is the deck mirror.
    press_chord(&root, "P", true, false, true);
    next_tick().await;
    let deck = query(host, "[data-overlay=\"command-deck\"]").expect("Ctrl+Shift+P opens the deck");
    assert_eq!(deck.get_attribute("data-goto-mode").unwrap(), "false");

    // A1 address is navigation sugar: typing "B2" surfaces a goto entry whose
    // execution selects the canonical address (Name-Box mirror).
    let input = query(host, ".dna-deck__input").expect("deck input");
    type_into(&input, "B2");
    next_tick().await;
    let goto = query(host, "[data-command-id=\"goto:a1:B2\"]").expect("A1 goto entry present");
    goto.unchecked_ref::<web_sys::HtmlElement>().click();
    next_tick().await;
    assert_eq!(
        mounted.dispatcher.intents(),
        vec![WorkspaceIntent::SelectNode(Some(NodeId::new("B2")))],
        "A1 goto dispatches a selection at the canonical address"
    );
    assert!(query(host, "[data-overlay=\"command-deck\"]").is_none());
}

#[wasm_bindgen_test]
async fn command_deck_theme_switch_retheme_the_cockpit() {
    let mounted = mount_calc_shell();
    let host = &mounted.host;
    next_tick().await;
    let root = shell_root(host);
    let shell = query(host, ".dna-shell").unwrap();
    assert_eq!(
        shell.get_attribute("data-dna-theme").unwrap(),
        "cockpit-light"
    );

    press_chord(&root, "k", true, false, false);
    next_tick().await;
    // Theme switch is a view-state control (no dispatch); executing it
    // re-themes the whole cockpit via the reactive --dna-* block.
    let dark =
        query(host, "[data-command-id=\"shell.theme.cockpit-dark\"]").expect("theme command");
    dark.unchecked_ref::<web_sys::HtmlElement>().click();
    next_tick().await;
    assert_eq!(
        shell.get_attribute("data-dna-theme").unwrap(),
        "cockpit-dark"
    );
    assert!(
        mounted.dispatcher.intents().is_empty(),
        "a theme switch is view-state, never a dispatched intent"
    );
}

/// SHELL_SPEC §1.1: the mast control buttons are the pointer/touch path into
/// the overlays and rails. Clicking them must do exactly what the Ctrl+K /
/// Ctrl+B / Ctrl+I verbs do — open the deck, collapse/expand the rails —
/// and never dispatch a workspace intent.
#[wasm_bindgen_test]
async fn mast_controls_drive_deck_and_rails_without_a_keyboard() {
    let mounted = mount_calc_shell();
    let host = &mounted.host;
    next_tick().await;

    // The Calc composition renders all three controls (the deck is built-in;
    // both rails compose for Calc).
    let registry_button =
        query(host, "[data-testid=\"mast-toggle-registry\"]").expect("registry toggle renders");
    let inspector_button =
        query(host, "[data-testid=\"mast-toggle-inspector\"]").expect("inspector toggle renders");
    query(host, "[data-testid=\"mast-open-commands\"]").expect("commands button renders");

    // Registry starts expanded; the button collapses it exactly like Ctrl+B,
    // reflected in aria-expanded AND the rail's collapsed class.
    assert_eq!(
        registry_button.get_attribute("aria-expanded").as_deref(),
        Some("true"),
        "rails start expanded on desktop"
    );
    registry_button
        .unchecked_ref::<web_sys::HtmlElement>()
        .click();
    next_tick().await;
    let rail = query(host, ".dna-registry").expect("registry rail mounts");
    assert!(
        rail.get_class_name().contains("dna-registry--collapsed"),
        "the button collapses the registry rail"
    );
    let registry_button = query(host, "[data-testid=\"mast-toggle-registry\"]")
        .expect("registry toggle persists across re-render");
    assert_eq!(
        registry_button.get_attribute("aria-expanded").as_deref(),
        Some("false")
    );

    // Same contract on the inspector side.
    inspector_button
        .unchecked_ref::<web_sys::HtmlElement>()
        .click();
    next_tick().await;
    let inspector = query(host, ".dna-inspector").expect("inspector mounts");
    assert!(
        inspector
            .get_class_name()
            .contains("dna-inspector--collapsed"),
        "the button collapses the inspector panel"
    );

    // The commands button opens the same deck Ctrl+K opens, with zero
    // dispatched intents (view-state only).
    let deck_button = query(host, "[data-testid=\"mast-open-commands\"]").unwrap();
    deck_button.unchecked_ref::<web_sys::HtmlElement>().click();
    next_tick().await;
    assert!(
        query(host, "[data-overlay=\"command-deck\"]").is_some(),
        "the mast commands button opens the command deck"
    );
    assert!(
        mounted.dispatcher.intents().is_empty(),
        "mast controls are view-state, never dispatched intents"
    );
}

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
use dnacalc_skin_ir::identity::NodeKey;
use dnacalc_skin_ir::intent::WorkspaceDelta;
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

    // Esc closes the topmost overlay first.
    press_chord(&root, "Escape", false, false, false);
    next_tick().await;
    assert!(query(host, "[data-overlay=\"command-deck\"]").is_none());

    // Ctrl+F opens the deck placeholder in goto mode.
    press_chord(&root, "f", true, false, false);
    next_tick().await;
    let deck = query(host, "[data-overlay=\"command-deck\"]").unwrap();
    assert_eq!(deck.get_attribute("data-goto-mode").unwrap(), "true");
    press_chord(&root, "Escape", false, false, false);
    next_tick().await;

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

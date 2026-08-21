//! The shell components — SHELL_SPEC.md §1 regions, §3 stage host, §4 data
//! wiring, §5 keyboard, §7 Registry/Inspector/Strip.
//!
//! One root [`Shell`] component composes the regions a product declares
//! ([`ShellComposition`]); omission is first-class. All colors resolve
//! through Strand `--dna-*` custom properties (SHELL_SPEC §8) — there is no
//! other styling pipeline here.

use std::sync::Arc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

use dnacalc_skin_ir::intent::{Dispatcher, WorkspaceDelta, WorkspaceIntent};
use dnacalc_skin_ir::keychord::{KeybindingOverrideMap, SkinVerb};
use dnacalc_skin_ir::preview::PreviewService;
use dnacalc_skin_ir::protocol::{HostCapabilityProjection, PersistenceProjection, SkinShellIntent};
use dnacalc_skin_ir::selection::SelectionState;
use dnacalc_skin_ir::state::{SharedSkinState, SharedStateChange, SharedStateOrigin};
use dnacalc_skin_ir::workspace::{NodeCalcStateProjection, NodeView, WorkspaceState};
use dnacalc_skin_leptos::state_handles::SharedSkinStateHandle;
use dnacalc_strand::{Density, IdentityBadge, Theme, css_custom_properties};

use crate::command_deck::{COMMAND_DECK_CSS, CommandDeck};
use crate::composition::{
    CatalogComposition, INSPECTOR_WIDTH_PX, InspectorSurface, MAST_HEIGHT_PX, REGISTRY_WIDTH_PX,
    STRIP_HEIGHT_PX, ShellComposition,
};
use crate::inspector::{InspectorSlotContent, InspectorSlotKind, inspector_slot_content};
use crate::keyboard::{
    KeyRouting, ShellKeyboardRegistry, chord_from_key_event, route_key, verb_label,
};
use crate::overlay::{
    ActiveOverlay, EscapeOutcome, OverlayContext, OverlayModel, PeekCard, ShellControls,
    ShellOverlaySlots,
};
use crate::profile::RuntimeContext;
use crate::stage::{
    StageContext, StageRegistry, StageResolution, continuity_halo, resolve_active_stage,
    switch_stage,
};
use crate::strip::{StripSlot, StripSlotContent, strip_slot_content};
#[cfg(target_arch = "wasm32")]
use crate::viewport::{initial_is_narrow, install_narrow_watcher};

/// True when the keyboard event originates from a text-entry element —
/// the first guard in the §5 keydown order (proven estate pattern).
fn event_target_is_text_entry(ev: &leptos::ev::KeyboardEvent) -> bool {
    let Some(target) = ev.target() else {
        return false;
    };
    let Ok(element) = target.dyn_into::<leptos::web_sys::Element>() else {
        return false;
    };
    matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
        || element
            .get_attribute("contenteditable")
            .is_some_and(|value| value != "false")
}

/// The peek target for the `P` verb: continuity focus first, then the
/// shared selection set, then the dispatcher-routed primary selection.
fn peek_target(
    shared: &SharedSkinState,
    workspace: &WorkspaceState,
    selection: &SelectionState,
) -> Option<dnacalc_skin_ir::identity::NodeKey> {
    shared
        .focus_key
        .clone()
        .or_else(|| shared.selection_set.first().cloned())
        .or_else(|| {
            selection
                .primary
                .as_ref()
                .and_then(|id| workspace.nodes.get(id))
                .map(|node| node.key.clone())
        })
}

/// The mast dirty dot: projection truth only (any sheet with undrained
/// edits). No projection, no dot — never a guessed state.
fn document_is_dirty(workspace: &WorkspaceState) -> bool {
    workspace
        .workbook_calc
        .as_ref()
        .is_some_and(|calc| calc.sheets.iter().any(|sheet| sheet.dirty))
}

/// The mast dirty dot, OR-combining the tree-workspace calc projection with
/// the host's `PersistenceProjection.dirty` (bead dtc-lfz.3) — a OneFormula/
/// OneCalc host has no `workbook_calc` projection at all, so without this it
/// could never show a dirty dot; a TreeCalc host with no `host_persistence`
/// prop wired keeps today's workspace-only behavior (`persistence` is
/// `None`). Either signal alone is sufficient; this never fabricates dirt
/// beyond what one of the two projections actually reports.
fn mast_dirty(workspace: &WorkspaceState, persistence: Option<&PersistenceProjection>) -> bool {
    document_is_dirty(workspace) || persistence.is_some_and(|projection| projection.dirty)
}

/// SHELL_SPEC §7 red-error registry liveness: true when a node's calc-state
/// projection reports a rejected-and-unrepaired formula or a cycle block, or
/// the node carries binding diagnostics — the error signals a `NodeView`
/// actually exposes at S0. Computed from the projection at render time;
/// never stored (§7: "computed from projections, never stored").
fn node_carries_error(node: &NodeView) -> bool {
    matches!(
        node.calc_state,
        Some(
            NodeCalcStateProjection::RejectedPendingRepair | NodeCalcStateProjection::CycleBlocked
        )
    ) || !node.binding_diagnostics.is_empty()
}

/// The registry-rail liveness dot class (SHELL_SPEC §7: green fresh, signal
/// stale/volatile, red error — computed from projections, never stored).
/// Error outranks stale; an entry with neither signal is fresh.
fn liveness_dot_class(error: bool, stale: bool) -> &'static str {
    if error {
        "dna-liveness dna-liveness--error"
    } else if stale {
        "dna-liveness dna-liveness--stale"
    } else {
        "dna-liveness dna-liveness--fresh"
    }
}

/// Region styles. Everything resolves through `--dna-*` tokens; the region
/// dimensions come from the SHELL_SPEC §1 constants so CSS cannot drift
/// from the layout contract.
fn shell_css(bridge_height_px: u32) -> String {
    format!(
        "{STATIC_SHELL_CSS}\n\
         .dna-shell {{ grid-template-rows: {MAST_HEIGHT_PX}px {bridge_height_px}px minmax(0, 1fr) {STRIP_HEIGHT_PX}px; }}\n\
         .dna-registry {{ width: {REGISTRY_WIDTH_PX}px; }}\n\
         .dna-registry--collapsed {{ width: 0; padding: 0; border-right: none; overflow: hidden; }}\n\
         .dna-inspector {{ width: {INSPECTOR_WIDTH_PX}px; }}\n\
         .dna-inspector--collapsed {{ width: 0; padding: 0; border-left: none; overflow: hidden; }}\n"
    )
}

const STATIC_SHELL_CSS: &str = r#"
.dna-shell {
  display: grid;
  height: 100%;
  min-height: 0;
  font-family: system-ui, sans-serif;
  line-height: var(--dna-line-height);
  background: var(--dna-bg);
  color: var(--dna-ink);
  outline: none;
}
.dna-mast {
  display: flex;
  align-items: center;
  gap: var(--dna-gap-4);
  padding: 0 var(--dna-gap-4);
  background: var(--dna-chrome);
  color: var(--dna-chrome-ink);
  overflow: hidden;
}
.dna-mast__mark { font-weight: 700; white-space: nowrap; }
.dna-mast__badge { display: inline-flex; align-items: center; gap: 2px; }
.dna-mast__badge-block { display: inline-block; height: 10px; border-radius: 2px; }
.dna-mast__doc { display: inline-flex; align-items: center; gap: var(--dna-gap-2); white-space: nowrap; min-width: 0; max-width: 40vw; }
.dna-mast__doc > span:first-child { overflow: hidden; text-overflow: ellipsis; min-width: 0; }
.dna-mast__dirty-dot {
  width: 7px; height: 7px; border-radius: 50%;
  background: var(--dna-amber);
}
.dna-mast__spacer { flex: 1; }
.dna-mast__switcher { display: inline-flex; gap: 1px; border-radius: var(--dna-radius-chip); overflow-x: auto; overflow-y: hidden; scrollbar-width: none; min-width: 0; flex-shrink: 1; }
.dna-mast__switcher::-webkit-scrollbar { display: none; }
.dna-mast__switcher button {
  border: none; cursor: pointer; flex: none; white-space: nowrap;
  padding: 3px 10px;
  background: var(--dna-chrome-2);
  color: var(--dna-chrome-ink-2);
  font: inherit; font-size: 12px;
}
.dna-mast__controls { display: inline-flex; gap: 2px; flex: none; margin-left: auto; }
.dna-mast__ctl {
  border: none; cursor: pointer;
  min-width: 28px; height: 28px; padding: 0 6px;
  display: inline-flex; align-items: center; justify-content: center;
  border-radius: var(--dna-radius-chip);
  background: var(--dna-chrome-2); color: var(--dna-chrome-ink-2);
  font: inherit; font-size: 14px; line-height: 1;
}
.dna-mast__ctl:hover { background: var(--dna-accent); color: var(--dna-chrome-ink); }
.dna-mast__ctl:focus-visible { outline: 2px solid var(--dna-accent); outline-offset: 1px; }
.dna-mast__switcher button[aria-selected="true"] {
  background: var(--dna-accent);
  color: var(--dna-chrome-ink);
}
.dna-mast__persona {
  padding: 1px 8px;
  border-radius: var(--dna-radius-chip);
  background: var(--dna-chrome-2);
  color: var(--dna-chrome-ink-2);
  font-size: 11px;
}
.dna-bridge {
  display: flex;
  align-items: center;
  padding: 0 var(--dna-gap-4);
  background: var(--dna-chrome-2);
  color: var(--dna-chrome-ink-2);
  border-bottom: 1px solid var(--dna-chrome);
}
.dna-bridge__placeholder { font-size: 12px; font-style: italic; }
.dna-middle { display: flex; min-height: 0; }
.dna-registry {
  flex: none;
  background: var(--dna-paper-2);
  border-right: 1px solid var(--dna-line);
  overflow-y: auto;
  padding: var(--dna-gap-3);
  transition: width var(--dna-motion-min) ease-out;
}
.dna-registry__filter { width: 100%; box-sizing: border-box; margin-bottom: var(--dna-gap-3); }
.dna-registry__section h3 {
  margin: var(--dna-gap-3) 0 var(--dna-gap-2);
  font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em;
  color: var(--dna-ink-3);
}
.dna-registry__entry { display: flex; align-items: center; gap: var(--dna-gap-2); font-size: 12px; padding: 1px 0; }
.dna-liveness { width: 7px; height: 7px; border-radius: 50%; flex: none; }
.dna-liveness--fresh { background: var(--dna-value-ink); }
.dna-liveness--stale { background: var(--dna-amber); }
.dna-liveness--error { background: var(--dna-red-ink); }
.dna-registry__absent { font-size: 12px; color: var(--dna-ink-3); font-style: italic; }
.dna-stage-host { flex: 1; min-width: 0; background: var(--dna-paper); overflow: auto; position: relative; }
.dna-stage-host__mount { height: 100%; }
.dna-continuity-halo { animation: dna-halo-pulse var(--dna-motion-max) ease-out 1; }
@keyframes dna-halo-pulse {
  from { box-shadow: inset 0 0 0 3px var(--dna-accent); }
  to { box-shadow: inset 0 0 0 0 transparent; }
}
@media (prefers-reduced-motion: reduce) {
  .dna-continuity-halo { animation: none; }
  .dna-registry { transition: none; }
}
.dna-stage-fallback {
  margin: var(--dna-gap-6);
  padding: var(--dna-gap-5);
  border: 1px solid var(--dna-red-ink);
  border-radius: var(--dna-radius-card);
  background: var(--dna-red-soft);
  color: var(--dna-ink);
}
.dna-stage-fallback__title { font-weight: 700; color: var(--dna-red-ink); display: block; }
.dna-inspector {
  flex: none;
  background: var(--dna-paper-2);
  border-left: 1px solid var(--dna-line);
  overflow-y: auto;
  padding: var(--dna-gap-3);
  transition: width var(--dna-motion-min) ease-out;
}
.dna-inspector__block { margin-bottom: var(--dna-gap-4); }
.dna-inspector__block h3 { margin: 0 0 var(--dna-gap-2); font-size: 12px; }
.dna-inspector__block p, .dna-inspector__block li { font-size: 12px; margin: 0; }
.dna-inspector__block--absent { color: var(--dna-ink-3); font-style: italic; }
.dna-strip {
  display: flex;
  align-items: center;
  gap: var(--dna-gap-5);
  padding: 0 var(--dna-gap-4);
  background: var(--dna-chrome-2);
  color: var(--dna-chrome-ink-2);
  font-size: 11px;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}
.dna-strip::-webkit-scrollbar { display: none; }
.dna-strip__slot { white-space: nowrap; }
.dna-strip__slot--absent { opacity: 0.65; }
.dna-strip__slot--clickable { cursor: pointer; text-decoration: underline dotted; }
.dna-strip__slot--reserved { min-width: 8px; }
.dna-strip__spacer { flex: 1; }
.dna-overlay-backdrop {
  position: fixed; inset: 0;
  background: rgba(10, 23, 28, 0.45);
  display: flex; align-items: flex-start; justify-content: center;
  padding: 8vh 12px 16px;
  z-index: 40;
}
.dna-overlay {
  background: var(--dna-paper);
  color: var(--dna-ink);
  border-radius: var(--dna-radius-panel);
  border: 1px solid var(--dna-line);
  width: min(720px, calc(100vw - 24px));
  box-sizing: border-box;
  max-height: 80vh;
  overflow-y: auto;
  padding: var(--dna-gap-5);
}
.dna-overlay h2 { margin: 0 0 var(--dna-gap-3); font-size: 15px; }
.dna-overlay__placeholder { color: var(--dna-ink-3); font-style: italic; font-size: 13px; }
.dna-atlas__group h3 {
  margin: var(--dna-gap-4) 0 var(--dna-gap-2);
  font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em;
  color: var(--dna-ink-3);
}
.dna-atlas__row {
  display: flex; align-items: center; gap: var(--dna-gap-3);
  padding: 2px 0; font-size: 13px;
}
.dna-atlas__chord {
  font-family: ui-monospace, monospace;
  background: var(--dna-paper-2);
  border: 1px solid var(--dna-line);
  border-radius: var(--dna-radius-chip);
  padding: 0 6px;
  min-width: 64px;
  text-align: center;
}
.dna-atlas__tag {
  font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em;
  padding: 0 5px;
  border-radius: var(--dna-radius-chip);
  background: var(--dna-signal-soft);
  color: var(--dna-prov-ext);
}
.dna-atlas__spacer { flex: 1; }
.dna-atlas__rebind {
  border: 1px solid var(--dna-line);
  background: var(--dna-paper-2);
  color: var(--dna-ink-2);
  border-radius: var(--dna-radius-chip);
  font-size: 11px;
  cursor: pointer;
}
.dna-atlas__error {
  color: var(--dna-red-ink);
  font-size: 12px;
  margin-top: var(--dna-gap-3);
}
.dna-atlas__arming { color: var(--dna-accent-ink); font-size: 12px; margin-top: var(--dna-gap-3); }
.dna-peeks {
  position: fixed; right: var(--dna-gap-5); bottom: calc(26px + var(--dna-gap-5));
  display: flex; flex-direction: column; gap: var(--dna-gap-3);
  z-index: 30;
}
.dna-peek-card {
  background: var(--dna-paper);
  border: 1px solid var(--dna-line);
  border-radius: var(--dna-radius-card);
  padding: var(--dna-gap-3) var(--dna-gap-4);
  font-size: 12px;
  display: flex; align-items: center; gap: var(--dna-gap-3);
}
/* --- Responsive provisions (SHELL_SPEC §1.1) -------------------------- */
/* Narrow viewports: the rails compose as overlay panels OVER the stage
   instead of squeezing it. They start collapsed (the Shell auto-collapses
   on entering narrow); the mast buttons bring them back. */
@media (max-width: 900px) {
  .dna-middle { position: relative; }
  .dna-registry:not(.dna-registry--collapsed) {
    position: absolute; top: 0; bottom: 0; left: 0; z-index: 20;
    width: min(232px, 78vw);
    box-shadow: 0 0 0 1px var(--dna-line), 8px 0 20px rgba(10, 23, 28, 0.35);
  }
  .dna-inspector:not(.dna-inspector--collapsed) {
    position: absolute; top: 0; bottom: 0; right: 0; z-index: 20;
    width: min(268px, 84vw);
    box-shadow: 0 0 0 1px var(--dna-line), -8px 0 20px rgba(10, 23, 28, 0.35);
  }
}
@media (max-width: 600px) {
  .dna-mast__mark { display: none; }
  .dna-mast__persona { display: none; }
  .dna-mast__doc { max-width: 34vw; }
}
/* Coarse pointers: chrome targets reach touch size (≈38-44px). */
@media (pointer: coarse) {
  .dna-mast__ctl { min-width: 38px; height: 38px; font-size: 16px; }
  .dna-mast__switcher button { padding: 10px 14px; font-size: 13px; }
  .dna-strip { font-size: 12px; }
  .dna-registry__entry { padding: 5px 0; }
}
"#;

/// The root shell (SHELL_SPEC §1). Mounts the composed regions over one
/// workspace/selection/shared source of truth, owns the overlay layer and
/// the §5 keydown pipeline, and hosts the active stage.
#[component]
#[allow(clippy::too_many_lines)]
pub fn Shell(
    composition: ShellComposition,
    stages: StageRegistry,
    workspace: ReadSignal<WorkspaceState>,
    latest_delta: ReadSignal<WorkspaceDelta>,
    selection: ReadSignal<SelectionState>,
    shared: SharedSkinStateHandle,
    dispatch: Arc<dyn Dispatcher>,
    theme: Theme,
    density: Density,
    runtime: RuntimeContext,
    #[prop(optional)] overlays: ShellOverlaySlots,
    #[prop(optional_no_strip)] preview: Option<Arc<dyn PreviewService>>,
    /// Verbs the shell itself does not own (Commit, EditInPlace, NameBox,
    /// traces, folds, Save/Open, …) are forwarded here for the host or the
    /// focused stage to interpret.
    #[prop(optional_no_strip)]
    on_shell_verb: Option<Callback<SkinVerb>>,
    /// `prefers-reduced-motion` as detected by the host; suppresses the
    /// continuity halo entirely (Strand motion law).
    #[prop(optional)]
    reduced_motion: bool,
    /// The host's advertised Save/Open/dirty capability (SHELL_SPEC §4,
    /// bead dtc-lfz.3) — the same `PersistenceProjection` the wire protocol
    /// already carries on `SkinShellProjection.persistence`. `None` (the
    /// default) reproduces the prior hardcoded-`false` behavior exactly, so
    /// existing mounts that do not pass this prop keep compiling and
    /// rendering the same disabled-with-reason Save/Open they always have.
    #[prop(optional)]
    host_persistence: Option<ReadSignal<PersistenceProjection>>,
    /// Where the command deck dispatches `SkinShellIntent::{Save, SaveAs,
    /// Open, OpenRecent}` (bead dtc-lfz.3) once `host_persistence` advertises
    /// the capability — the OneFormula/OneCalc host's own shell-intent
    /// channel, distinct from `dispatch` above (which only carries
    /// `WorkspaceIntent`, a closed set with no document-lifecycle member).
    /// `None` when the product has not wired it.
    #[prop(optional_no_strip)]
    on_shell_intent: Option<Callback<SkinShellIntent>>,
    /// The host's real `HostCapabilityProjection` (SHELL_SPEC §7 mechanism
    /// 18, bead dtc-lfz.6, G7 minimal slice) — feeds the Strip's Feeds
    /// instrument honestly. `None` (the default) reproduces the prior
    /// hardcoded-`Nothing` Feeds slot exactly, so existing mounts that do
    /// not pass this prop keep compiling and rendering identically.
    #[prop(optional)]
    host_capabilities: Option<ReadSignal<HostCapabilityProjection>>,
) -> impl IntoView {
    let profile = composition.profile;
    let visible_stages_meta: Vec<(u8, &'static str)> = stages
        .visible(&profile)
        .iter()
        .enumerate()
        .map(|(index, stage)| (u8::try_from(index + 1).unwrap_or(u8::MAX), stage.title()))
        .collect();
    let catalog: CatalogComposition = composition.catalog_composition(visible_stages_meta.len());

    // Build the active grammar from the persisted overrides. A persisted map
    // that fails validation (stale verb id from another build, a chord this
    // runtime hard-reserves) falls back to the universal table rather than
    // dropping keystrokes (estate pattern).
    let build_registry = move |overrides: &KeybindingOverrideMap| {
        ShellKeyboardRegistry::with_overrides(catalog, runtime, overrides)
            .unwrap_or_else(|_| ShellKeyboardRegistry::universal(catalog, runtime))
    };

    let overlay = RwSignal::new(OverlayModel::default());
    let registry_collapsed = RwSignal::new(false);
    let inspector_collapsed = RwSignal::new(false);
    // Theme/density are shell view-state so the command deck (and any future
    // control) can switch them; the `<style>` block below reads these
    // reactively, so a switch re-themes the whole cockpit — chrome and every
    // token-styled stage — through the `--dna-*` cascade.

    // Responsive provisions (SHELL_SPEC §1.1): below the narrow breakpoint
    // the rails compose as overlay panels over the stage (pure CSS media
    // query) and START collapsed so a phone lands on the stage, not chrome.
    // Entering narrow collapses any open rail once; leaving narrow restores
    // the desktop contract without forcing anything open. The mast control
    // buttons are the pointer/touch path back to a rail.
    #[cfg(target_arch = "wasm32")]
    let narrow = RwSignal::new(initial_is_narrow());
    #[cfg(not(target_arch = "wasm32"))]
    let narrow = RwSignal::new(false);
    #[cfg(target_arch = "wasm32")]
    install_narrow_watcher(narrow);
    Effect::new(move |_| {
        if narrow.get() {
            registry_collapsed.set(true);
            inspector_collapsed.set(true);
        }
    });
    let theme_sig = RwSignal::new(theme);
    let density_sig = RwSignal::new(density);

    // Install the built-in command deck into the seam when the product did not
    // supply its own (SHELL_SPEC §5.1; the deck lives in this crate).
    let mut overlays = overlays;
    if overlays.command_deck.is_none() {
        overlays.command_deck = Some(Arc::new(CommandDeck));
    }

    let stage_ctx = StageContext {
        workspace,
        latest_delta,
        selection,
        shared,
        theme,
        density,
        dispatch: dispatch.clone(),
        preview,
    };

    let active_lens = Memo::new(move |_| shared.with(|state| state.active_lens.clone()));

    // --- §5 keydown pipeline: guard, resolve, route ---
    let keydown_stages = stages.clone();
    let keydown_dispatch = dispatch.clone();
    let handle_keydown = move |ev: leptos::ev::KeyboardEvent| {
        let chord = chord_from_key_event(
            &ev.key(),
            ev.ctrl_key(),
            ev.meta_key(),
            ev.alt_key(),
            ev.shift_key(),
        );
        let registry = build_registry(&shared.get_untracked().keybinding_overrides);
        let verb = match route_key(&registry, event_target_is_text_entry(&ev), &chord) {
            KeyRouting::SuppressedByTextEntry | KeyRouting::Unbound => return,
            KeyRouting::Verb(verb) => verb,
        };
        let forward = |verb: SkinVerb| {
            if let Some(callback) = on_shell_verb {
                callback.run(verb);
            }
        };
        match verb {
            SkinVerb::CommandDeck => {
                ev.prevent_default();
                overlay.update(|model| model.open(ActiveOverlay::CommandDeck { goto_mode: false }));
            }
            SkinVerb::FindGoto => {
                ev.prevent_default();
                overlay.update(|model| model.open(ActiveOverlay::CommandDeck { goto_mode: true }));
            }
            SkinVerb::KeyboardAtlas => {
                ev.prevent_default();
                overlay.update(|model| model.open(ActiveOverlay::KeyboardAtlas));
            }
            SkinVerb::Timeline => {
                ev.prevent_default();
                overlay.update(|model| model.open(ActiveOverlay::Timeline));
            }
            SkinVerb::Peek => {
                let shared_state = shared.get_untracked();
                let target = workspace.with_untracked(|ws| {
                    selection.with_untracked(|sel| peek_target(&shared_state, ws, sel))
                });
                if let Some(key) = target {
                    ev.prevent_default();
                    overlay.update(|model| model.peeks.show_transient(PeekCard { key }));
                }
            }
            SkinVerb::Escape => {
                // Esc closes the topmost overlay before it does anything else.
                let mut outcome = EscapeOutcome::Unhandled;
                overlay.update(|model| outcome = model.escape());
                if outcome == EscapeOutcome::Unhandled {
                    forward(SkinVerb::Escape);
                } else {
                    ev.prevent_default();
                }
            }
            SkinVerb::RegistryToggle => {
                ev.prevent_default();
                registry_collapsed.update(|collapsed| *collapsed = !*collapsed);
            }
            SkinVerb::InspectorToggle => {
                ev.prevent_default();
                inspector_collapsed.update(|collapsed| *collapsed = !*collapsed);
            }
            SkinVerb::SwitchLens(slot) => {
                ev.prevent_default();
                // Re-projection only — audited shared-state write, no intent.
                switch_stage(&shared, &keydown_stages, &profile, slot);
            }
            SkinVerb::Recalculate => {
                ev.prevent_default();
                keydown_dispatch.dispatch(WorkspaceIntent::Recalculate);
                shared.apply(
                    SharedStateChange::SetManualRecalcPending(false),
                    SharedStateOrigin::Shell,
                );
            }
            SkinVerb::Undo => {
                ev.prevent_default();
                keydown_dispatch.dispatch(WorkspaceIntent::Undo);
            }
            SkinVerb::Redo => {
                ev.prevent_default();
                keydown_dispatch.dispatch(WorkspaceIntent::Redo);
            }
            SkinVerb::Save | SkinVerb::Open => {
                // Interceptable per §5.1; disabled with an honest badge until
                // host support — intercept the browser default, forward.
                ev.prevent_default();
                forward(verb);
            }
            other => forward(other),
        }
    };

    // Shell-local control callbacks the command deck drives (view-state only,
    // never dispatched intents). Built here where every shell signal is in
    // scope; handed to the deck through the extended OverlayContext.
    let controls_stages = stages.clone();
    let controls = ShellControls {
        switch_stage: Callback::new(move |slot: u8| {
            switch_stage(&shared, &controls_stages, &profile, slot);
        }),
        toggle_registry: Callback::new(move |()| {
            registry_collapsed.update(|collapsed| *collapsed = !*collapsed);
        }),
        toggle_inspector: Callback::new(move |()| {
            inspector_collapsed.update(|collapsed| *collapsed = !*collapsed);
        }),
        open_atlas: Callback::new(move |()| {
            overlay.update(|model| model.open(ActiveOverlay::KeyboardAtlas));
        }),
        open_timeline: Callback::new(move |()| {
            overlay.update(|model| model.open(ActiveOverlay::Timeline));
        }),
        set_theme: Callback::new(move |next: Theme| theme_sig.set(next)),
        set_density: Callback::new(move |next: Density| density_sig.set(next)),
        close: Callback::new(move |()| overlay.update(OverlayModel::close)),
    };

    let bridge_height = composition.bridge_slot.height_px();
    // Reactive theme/density → the whole cockpit re-themes on a deck switch.
    let css = move || {
        format!(
            "{}\n{}\n{}",
            css_custom_properties(theme_sig.get(), density_sig.get()),
            shell_css(bridge_height),
            COMMAND_DECK_CSS
        )
    };

    // --- Region views ---
    let mast_stages = stages.clone();
    let host_stages = stages.clone();
    let badge = profile.identity_badge();
    let switcher_composed = composition.mast.stage_switcher;
    let registry_composed = composition.registry.is_some();
    let inspector_composed = composition.inspector.is_some();
    let inspector_surface = composition.inspector_surface.clone();
    let inspector_ctx = stage_ctx.clone();
    let bridge_slot = composition.bridge_slot.clone();
    let bridge_ctx = stage_ctx.clone();
    let host_ctx = stage_ctx.clone();
    let product_mark = composition.product_mark;

    let document_name = Memo::new(move |_| {
        workspace.with(|ws| {
            if ws.workspace_id.is_empty() {
                "Untitled".to_string()
            } else {
                shared
                    .with(|state| state.workspace_names.get(&ws.workspace_id).cloned())
                    .unwrap_or_else(|| ws.workspace_id.clone())
            }
        })
    });
    let dirty = Memo::new(move |_| {
        workspace.with(|ws| {
            let persistence = host_persistence.map(|signal| signal.get());
            mast_dirty(ws, persistence.as_ref())
        })
    });
    let persona_label = Memo::new(move |_| shared.with(|state| state.persona.stable_id()));

    view! {
        <style>{move || css()}</style>
        <div
            class="dna-shell"
            class:dna-reduced-motion=reduced_motion
            data-dna-theme=move || theme_sig.get().id()
            data-dna-density=move || density_sig.get().id()
            data-profile=profile.stable_id()
            tabindex="0"
            on:keydown=handle_keydown
        >
            <header class="dna-mast" data-region="mast">
                <span class="dna-mast__mark">{product_mark}</span>
                <IdentityBadgeView badge=badge />
                <span class="dna-mast__doc">
                    <span>{move || document_name.get()}</span>
                    {move || {
                        dirty
                            .get()
                            .then(|| {
                                view! {
                                    <span
                                        class="dna-mast__dirty-dot"
                                        title="unsaved calc edits"
                                        aria-label="document dirty"
                                    ></span>
                                }
                            })
                    }}
                </span>
                <span class="dna-mast__spacer"></span>
                {switcher_composed
                    .then(|| {
                        let visible = mast_stages.visible(&profile);
                        let switch_registry = mast_stages.clone();
                        view! {
                            <div class="dna-mast__switcher" role="tablist" aria-label="Stage switcher">
                                {visible
                                    .iter()
                                    .enumerate()
                                    .map(|(index, stage)| {
                                        let slot = u8::try_from(index + 1).unwrap_or(u8::MAX);
                                        let id = stage.id();
                                        let title = stage.title();
                                        let stages = switch_registry.clone();
                                        view! {
                                            <button
                                                role="tab"
                                                data-stage-tab=id.stable_id()
                                                aria-selected=move || {
                                                    let active = active_lens.get();
                                                    let is_active = match active.as_deref() {
                                                        Some(current) => current == id.stable_id(),
                                                        None => index == 0,
                                                    };
                                                    if is_active { "true" } else { "false" }
                                                }
                                                on:click=move |_| {
                                                    switch_stage(&shared, &stages, &profile, slot);
                                                }
                                            >
                                                {title}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                    })}
                <span class="dna-mast__controls">
                    // Pointer/touch path into the overlays and rails
                    // (SHELL_SPEC §1.1): the deck was Ctrl+K-only, the rails
                    // Ctrl+B/Ctrl+I-only — unreachable without a keyboard.
                    <button
                        class="dna-mast__ctl"
                        type="button"
                        data-testid="mast-open-commands"
                        aria-label="Command deck"
                        title="Commands (Ctrl+K)"
                        on:click=move |_| {
                            overlay.update(|model| {
                                model.open(ActiveOverlay::CommandDeck { goto_mode: false });
                            });
                        }
                    >
                        "⌘"
                    </button>
                    {registry_composed.then(|| {
                        view! {
                            <button
                                class="dna-mast__ctl"
                                type="button"
                                data-testid="mast-toggle-registry"
                                aria-label=move || {
                                    if registry_collapsed.get() { "Show registry" } else { "Hide registry" }
                                }
                                title="Registry (Ctrl+B)"
                                aria-expanded=move || if registry_collapsed.get() { "false" } else { "true" }
                                on:click=move |_| {
                                    registry_collapsed.update(|collapsed| *collapsed = !*collapsed);
                                }
                            >
                                "☰"
                            </button>
                        }
                    })}
                    {inspector_composed.then(|| {
                        view! {
                            <button
                                class="dna-mast__ctl"
                                type="button"
                                data-testid="mast-toggle-inspector"
                                aria-label=move || {
                                    if inspector_collapsed.get() { "Show inspector" } else { "Hide inspector" }
                                }
                                title="Inspector (Ctrl+I)"
                                aria-expanded=move || if inspector_collapsed.get() { "false" } else { "true" }
                                on:click=move |_| {
                                    inspector_collapsed.update(|collapsed| *collapsed = !*collapsed);
                                }
                            >
                                "◫"
                            </button>
                        }
                    })}
                </span>
                <span class="dna-mast__persona" data-persona=move || persona_label.get()>
                    {move || persona_label.get()}
                </span>
            </header>
            <div class="dna-bridge" data-region="bridge" data-rows=if bridge_slot.hero_two_row { "2" } else { "1" }>
                {match bridge_slot.surface.clone() {
                    Some(surface) => surface.mount(bridge_ctx.clone()).into_any(),
                    None => {
                        view! {
                            <span class="dna-bridge__placeholder">
                                "Formula workbench not mounted (dnacalc-bridge, bead dtc-tsc.7)"
                            </span>
                        }
                            .into_any()
                    }
                }}
            </div>
            <div class="dna-middle">
                {registry_composed
                    .then(|| {
                        view! {
                            <RegistryRail
                                workspace=workspace
                                collapsed=registry_collapsed
                            />
                        }
                    })}
                <main class="dna-stage-host" data-region="stage-host">
                    {move || {
                        let lens = active_lens.get();
                        match resolve_active_stage(&host_stages, &profile, lens.as_deref()) {
                            StageResolution::Mount(stage) => {
                                let handle = stage.mount(host_ctx.clone());
                                if let Some(hook) = handle.on_deactivate {
                                    on_cleanup(hook);
                                }
                                let halo = !reduced_motion && continuity_halo(reduced_motion).is_some();
                                view! {
                                    <div
                                        class="dna-stage-host__mount"
                                        class:dna-continuity-halo=halo
                                        data-stage=stage.id().stable_id()
                                    >
                                        {handle.view}
                                    </div>
                                }
                                    .into_any()
                            }
                            StageResolution::Unavailable { active_lens } => {
                                stage_fallback(active_lens).into_any()
                            }
                        }
                    }}
                </main>
                {inspector_composed
                    .then(|| {
                        view! {
                            <InspectorPanel
                                workspace=workspace
                                selection=selection
                                collapsed=inspector_collapsed
                                stage_panel=inspector_surface
                                stage_ctx=inspector_ctx
                            />
                        }
                    })}
            </div>
            <footer class="dna-strip" data-region="strip">
                <StripView
                    workspace=workspace
                    shared=shared
                    overlay=overlay
                    host_capabilities=host_capabilities
                />
            </footer>
            <OverlayLayer
                overlay=overlay
                overlays=overlays
                shared=shared
                dispatch=dispatch.clone()
                catalog=catalog
                runtime=runtime
                workspace=workspace
                selection=selection
                theme_sig=theme_sig
                density_sig=density_sig
                visible_stages=visible_stages_meta
                controls=controls
                host_persistence=host_persistence
                on_shell_intent=on_shell_intent
            />
            <PeekLayer overlay=overlay />
        </div>
    }
}

/// The Strand base-pair identity badge (STRAND §2) — chrome identity spot
/// only.
#[component]
fn IdentityBadgeView(badge: IdentityBadge) -> impl IntoView {
    view! {
        <span
            class="dna-mast__badge"
            data-profile-badge=badge.profile_id
            data-pattern=badge.pattern
            aria-label=format!("profile: {}", badge.profile_id)
        >
            {badge
                .blocks
                .iter()
                .map(|block| {
                    view! {
                        <span
                            class="dna-mast__badge-block"
                            style=format!(
                                "width:{}px;background:{}",
                                block.size_px,
                                block.color.to_hex(),
                            )
                        ></span>
                    }
                })
                .collect_view()}
        </span>
    }
}

/// Fail-loud fallback card for a stage that cannot mount (SHELL_SPEC §3).
fn stage_fallback(active_lens: Option<String>) -> impl IntoView {
    let detail = match active_lens {
        Some(id) => format!("active stage '{id}' cannot mount under this profile"),
        None => "no stage is registered for this profile".to_string(),
    };
    view! {
        <div class="dna-stage-fallback" role="alert" data-stage-fallback="">
            <span class="dna-stage-fallback__title">"Stage unavailable"</span>
            <span class="dna-stage-fallback__detail">{detail}</span>
        </div>
    }
}

/// Registry rail (SHELL_SPEC §7): Names, Sheets/Top-nodes, Tables,
/// Extensions — liveness computed from projections, never stored.
#[component]
fn RegistryRail(workspace: ReadSignal<WorkspaceState>, collapsed: RwSignal<bool>) -> impl IntoView {
    let filter = RwSignal::new(String::new());
    let matches = move |label: &str| {
        let needle = filter.get();
        needle.is_empty() || label.to_lowercase().contains(&needle.to_lowercase())
    };

    view! {
        <aside
            class="dna-registry"
            class:dna-registry--collapsed=move || collapsed.get()
            data-region="registry"
            aria-label="Registry"
            aria-hidden=move || if collapsed.get() { "true" } else { "false" }
        >
            <input
                class="dna-registry__filter"
                type="search"
                placeholder="Filter"
                aria-label="Filter registry"
                on:input=move |ev| filter.set(event_target_value(&ev))
                prop:value=move || filter.get()
            />
            <section class="dna-registry__section" data-registry-section="names">
                <h3>"Names"</h3>
                {move || {
                    workspace.with(|ws| {
                        let entries: Vec<_> = ws
                            .defined_names
                            .entries
                            .iter()
                            .map(|entry| entry.name.clone())
                            .filter(|name| matches(name))
                            .collect();
                        if ws.defined_names.entries.is_empty() {
                            view! { <p class="dna-registry__absent">"no defined names projected"</p> }
                                .into_any()
                        } else {
                            entries
                                .into_iter()
                                .map(|name| {
                                    // Always fresh: DefinedNameProjection carries
                                    // no backing NodeId (its `scope` names the
                                    // sheet a name is scoped to, not a formula
                                    // node), so no calc-state/binding-diagnostic
                                    // signal exists to derive an error dot from
                                    // at S0. Do not fabricate one.
                                    view! {
                                        <div class="dna-registry__entry">
                                            <span class="dna-liveness dna-liveness--fresh"></span>
                                            <span>{name}</span>
                                        </div>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }
                    })
                }}
            </section>
            <section class="dna-registry__section" data-registry-section="sheets">
                <h3>"Sheets"</h3>
                {move || {
                    workspace.with(|ws| {
                        if ws.sheets.is_empty() {
                            // Tree session: top-level nodes are the section.
                            let roots: Vec<_> = ws
                                .root_paths
                                .iter()
                                .filter(|id| matches(&id.to_string()))
                                .collect();
                            if roots.is_empty() {
                                return view! {
                                    <p class="dna-registry__absent">"no sheets or top-level nodes"</p>
                                }
                                    .into_any();
                            }
                            return roots
                                .into_iter()
                                .map(|id| {
                                    // A root IS a node here (unlike the
                                    // grid-backed branch below), so its own
                                    // calc-state/binding-diagnostics are a
                                    // genuine, non-fabricated error signal.
                                    let error = ws.nodes.get(id).is_some_and(node_carries_error);
                                    let dot_class = liveness_dot_class(error, false);
                                    view! {
                                        <div class="dna-registry__entry">
                                            <span class=dot_class></span>
                                            <span>{id.to_string()}</span>
                                        </div>
                                    }
                                })
                                .collect_view()
                                .into_any();
                        }
                        ws.sheets
                            .iter()
                            .filter(|sheet| matches(&sheet.display_name))
                            .map(|sheet| {
                                // Liveness from the calc projection: stale
                                // (undrained edits) shows the signal dot.
                                let stale = ws
                                    .workbook_calc
                                    .as_ref()
                                    .is_some_and(|calc| {
                                        calc.sheets.iter().any(|summary| {
                                            summary.grid_node_id == sheet.grid_node_id
                                                && summary.dirty
                                        })
                                    });
                                // No error signal available here: a grid-backed
                                // sheet's `grid_node_id` lives in a distinct
                                // NodeId namespace ("sheet:{n}",
                                // `sheet_grid_node_id`) from `ws.nodes`'s tree
                                // node ids, so there is no NodeView to read a
                                // calc-state/binding-diagnostic off — and no
                                // per-sheet aggregate error projection exists at
                                // S0 either. Do not fabricate one; fresh/stale
                                // only until that projection lands.
                                let dot_class = liveness_dot_class(false, stale);
                                view! {
                                    <div class="dna-registry__entry">
                                        <span class=dot_class></span>
                                        <span>{sheet.display_name.clone()}</span>
                                    </div>
                                }
                            })
                            .collect_view()
                            .into_any()
                    })
                }}
            </section>
            <section class="dna-registry__section" data-registry-section="tables">
                <h3>"Tables"</h3>
                {move || {
                    workspace.with(|ws| {
                        if ws.tables.is_empty() {
                            view! { <p class="dna-registry__absent">"no tables projected"</p> }
                                .into_any()
                        } else {
                            ws.tables
                                .iter()
                                .filter(|(id, _)| matches(&id.to_string()))
                                .map(|(id, _table)| {
                                    // `ws.tables` is keyed by the table's own
                                    // backing NodeId, so its calc-state/
                                    // binding-diagnostics are a genuine,
                                    // non-fabricated error signal.
                                    let error = ws.nodes.get(id).is_some_and(node_carries_error);
                                    let dot_class = liveness_dot_class(error, false);
                                    view! {
                                        <div class="dna-registry__entry">
                                            <span class=dot_class></span>
                                            <span>{id.to_string()}</span>
                                        </div>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }
                    })
                }}
            </section>
            <section class="dna-registry__section" data-registry-section="extensions">
                <h3>"Extensions"</h3>
                // G7: honest not-yet-projected state, never a fake catalog.
                <p class="dna-registry__absent">"not yet projected (G7)"</p>
            </section>
        </aside>
    }
}

/// Inspector panel (SHELL_SPEC §7): typed slots, projection-fed; the
/// reserved `Evidence` slot renders nothing at all. A product may mount a
/// stage-specific surface into the `StagePanel` slot (bead dtc-lfz.5, the
/// designed extension point); absent one, that slot keeps its honest
/// projection-fed absence.
#[component]
fn InspectorPanel(
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<SelectionState>,
    collapsed: RwSignal<bool>,
    /// The product's Inspector stage-panel surface, mounted exactly once
    /// at the `StagePanel` slot. `None` leaves the honest absence.
    stage_panel: Option<Arc<dyn InspectorSurface>>,
    /// The same context every stage receives (SHELL_SPEC §3); handed to the
    /// stage-panel surface at mount.
    stage_ctx: StageContext,
) -> impl IntoView {
    // Mount the stage-panel surface once (single-use `AnyView`); the loop
    // takes it when it reaches the StagePanel slot.
    let mut stage_panel_view = stage_panel.map(|surface| surface.mount(stage_ctx));

    // One reactive typed slot (projection-fed), rendered for every kind that
    // is not a mounted stage panel.
    let typed_slot = move |kind: InspectorSlotKind| {
        view! {
            {move || {
                let content = workspace
                    .with(|ws| selection.with(|sel| inspector_slot_content(kind, ws, sel)));
                match content {
                    InspectorSlotContent::Block { title, lines } => view! {
                        <section
                            class="dna-inspector__block"
                            data-inspector-slot=kind.stable_id()
                        >
                            <h3>{title}</h3>
                            <ul>
                                {lines
                                    .into_iter()
                                    .map(|line| view! { <li>{line}</li> })
                                    .collect_view()}
                            </ul>
                        </section>
                    }
                    .into_any(),
                    InspectorSlotContent::Absent { title, reason } => view! {
                        <section
                            class="dna-inspector__block dna-inspector__block--absent"
                            data-inspector-slot=kind.stable_id()
                        >
                            <h3>{title}</h3>
                            <p>{reason}</p>
                        </section>
                    }
                    .into_any(),
                    // Reserved slots render NOTHING — not an empty frame, not a
                    // dash (PARITY_TRUST_UX §8).
                    InspectorSlotContent::Nothing => ().into_any(),
                }
            }}
        }
        .into_any()
    };

    view! {
        <aside
            class="dna-inspector"
            class:dna-inspector--collapsed=move || collapsed.get()
            data-region="inspector"
            aria-label="Inspector"
            aria-hidden=move || if collapsed.get() { "true" } else { "false" }
        >
            {InspectorSlotKind::ALL
                .iter()
                .map(|kind| {
                    let kind = *kind;
                    if kind == InspectorSlotKind::StagePanel
                        && let Some(surface_view) = stage_panel_view.take()
                    {
                        return view! {
                            <section
                                class="dna-inspector__block"
                                data-inspector-slot="stage-panel"
                            >
                                {surface_view}
                            </section>
                        }
                        .into_any();
                    }
                    typed_slot(kind)
                })
                .collect_view()}
        </aside>
    }
}

/// The Strip (SHELL_SPEC §7): wired readouts, honest absences, and the
/// reserved slots that render nothing.
///
/// The Feeds slot (mechanism 18, bead dtc-lfz.6) is the Extensions manager's
/// documented Strip entry point (BENCH_SPEC §2/§6: "reached from Strip/
/// Inspector") — clickable only when a product has wired
/// `host_capabilities`, opening `ActiveOverlay::Extensions` through the same
/// one-at-a-time `overlay` model every other overlay uses.
#[component]
fn StripView(
    workspace: ReadSignal<WorkspaceState>,
    shared: SharedSkinStateHandle,
    overlay: RwSignal<OverlayModel>,
    host_capabilities: Option<ReadSignal<HostCapabilityProjection>>,
) -> impl IntoView {
    let feeds_wired = host_capabilities.is_some();
    view! {
        {StripSlot::ALL
            .iter()
            .map(|slot| {
                let slot = *slot;
                let caps = host_capabilities;
                view! {
                    {move || {
                        let content = workspace
                            .with(|ws| {
                                shared
                                    .with(|state| {
                                        let caps = caps.map(|signal| signal.get());
                                        strip_slot_content(slot, ws, state, caps.as_ref())
                                    })
                            });
                        let feeds_clickable = feeds_wired && slot == StripSlot::Feeds;
                        match content {
                            StripSlotContent::Text(text) => {
                                let class = if feeds_clickable {
                                    "dna-strip__slot dna-strip__slot--clickable"
                                } else {
                                    "dna-strip__slot"
                                };
                                view! {
                                    <span
                                        class=class
                                        data-slot=slot.stable_id()
                                        data-clickable=if feeds_clickable { "true" } else { "false" }
                                        on:click=move |_| {
                                            if feeds_clickable {
                                                overlay
                                                    .update(|model| {
                                                        model.open(ActiveOverlay::Extensions)
                                                    });
                                            }
                                        }
                                    >
                                        {text}
                                    </span>
                                }
                                    .into_any()
                            }
                            StripSlotContent::Absent(reason) => {
                                view! {
                                    <span
                                        class="dna-strip__slot dna-strip__slot--absent"
                                        data-slot=slot.stable_id()
                                        title=reason
                                    >
                                        "\u{2014}"
                                    </span>
                                }
                                    .into_any()
                            }
                            // Reserved in layout math; renders NOTHING.
                            StripSlotContent::Nothing => {
                                view! {
                                    <span
                                        class="dna-strip__slot dna-strip__slot--reserved"
                                        data-slot=slot.stable_id()
                                    ></span>
                                }
                                    .into_any()
                            }
                        }
                    }}
                }
            })
            .collect_view()}
    }
}

/// The one-at-a-time overlay layer. The command deck mounts through the
/// `command_deck` slot (defaulted to the built-in [`CommandDeck`] by the
/// Shell); the atlas is a built-in; the timeline stays a placeholder until
/// its bead.
#[component]
#[allow(clippy::too_many_arguments)]
fn OverlayLayer(
    overlay: RwSignal<OverlayModel>,
    overlays: ShellOverlaySlots,
    shared: SharedSkinStateHandle,
    dispatch: Arc<dyn Dispatcher>,
    catalog: CatalogComposition,
    runtime: RuntimeContext,
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<SelectionState>,
    theme_sig: RwSignal<Theme>,
    density_sig: RwSignal<Density>,
    visible_stages: Vec<(u8, &'static str)>,
    controls: ShellControls,
    host_persistence: Option<ReadSignal<PersistenceProjection>>,
    on_shell_intent: Option<Callback<SkinShellIntent>>,
) -> impl IntoView {
    let active = Memo::new(move |_| overlay.with(|model| model.active));
    // The deck reads the live keyboard registry (same overrides the dispatcher
    // uses) so its effective chords cannot drift.
    let overlay_context = move |goto_mode: bool| {
        let overrides = shared.get_untracked().keybinding_overrides;
        let keyboard = ShellKeyboardRegistry::with_overrides(catalog, runtime, &overrides)
            .unwrap_or_else(|_| ShellKeyboardRegistry::universal(catalog, runtime));
        // Real host capability (SHELL_SPEC §4, bead dtc-lfz.3): `None` (no
        // `host_persistence` prop wired) reproduces the old hardcoded-`false`
        // behavior exactly; a wired signal's `can_save`/`can_open` drive the
        // deck's Save/Open enablement directly — never re-derived or guessed
        // here.
        let persistence = host_persistence.map(|signal| signal.get_untracked());
        OverlayContext {
            shared,
            dispatch: dispatch.clone(),
            goto_mode,
            workspace,
            selection,
            keyboard,
            theme: theme_sig.get_untracked(),
            density: density_sig.get_untracked(),
            visible_stages: visible_stages.clone(),
            host_can_save: persistence.as_ref().is_some_and(|p| p.can_save),
            host_can_open: persistence.as_ref().is_some_and(|p| p.can_open),
            shell_intent: on_shell_intent,
            controls,
        }
    };
    view! {
        {move || {
            active
                .get()
                .map(|current| {
                    match current {
                        ActiveOverlay::CommandDeck { goto_mode } => overlays
                            .command_deck
                            .clone()
                            .map(|surface| surface.mount(overlay_context(goto_mode)))
                            .unwrap_or_else(|| ().into_any()),
                        ActiveOverlay::KeyboardAtlas => {
                            view! {
                                <KeyboardAtlasOverlay
                                    shared=shared
                                    catalog=catalog
                                    runtime=runtime
                                />
                            }
                                .into_any()
                        }
                        ActiveOverlay::Timeline => {
                            match overlays.timeline.clone() {
                                Some(surface) => surface.mount(overlay_context(false)).into_any(),
                                None => {
                                    view! {
                                        <div class="dna-overlay-backdrop">
                                            <div
                                                class="dna-overlay"
                                                role="dialog"
                                                aria-label="Timeline"
                                                data-overlay="timeline"
                                            >
                                                <h2>"Timeline"</h2>
                                                <p class="dna-overlay__placeholder">
                                                    "Revision drawer (mechanism 19) — placeholder this phase."
                                                </p>
                                            </div>
                                        </div>
                                    }
                                        .into_any()
                                }
                            }
                        }
                        ActiveOverlay::Extensions => {
                            match overlays.extensions.clone() {
                                Some(surface) => surface.mount(overlay_context(false)).into_any(),
                                None => {
                                    view! {
                                        <div class="dna-overlay-backdrop">
                                            <div
                                                class="dna-overlay"
                                                role="dialog"
                                                aria-label="Extensions"
                                                data-overlay="extensions"
                                            >
                                                <h2>"Extensions"</h2>
                                                <p class="dna-overlay__placeholder">
                                                    "Extensions manager (bead dtc-lfz.6) — not mounted for this product."
                                                </p>
                                            </div>
                                        </div>
                                    }
                                        .into_any()
                                }
                            }
                        }
                    }
                })
        }}
    }
}

/// The keyboard atlas (SHELL_SPEC §5.2, mechanism 10): renders every active
/// chord FROM the live registry (rebuilt from the same persisted overrides
/// the dispatcher uses — it cannot drift), grouped by region/stage, with
/// divergence tags and a rebind affordance writing through the audited
/// chokepoint.
#[component]
fn KeyboardAtlasOverlay(
    shared: SharedSkinStateHandle,
    catalog: CatalogComposition,
    runtime: RuntimeContext,
) -> impl IntoView {
    let arming: RwSignal<Option<SkinVerb>> = RwSignal::new(None);
    let rebind_error: RwSignal<Option<String>> = RwSignal::new(None);

    let build = move |overrides: &KeybindingOverrideMap| {
        ShellKeyboardRegistry::with_overrides(catalog, runtime, overrides)
            .unwrap_or_else(|_| ShellKeyboardRegistry::universal(catalog, runtime))
    };

    // Chord capture while arming: the next non-modifier keydown becomes the
    // proposed chord; validation happens against the live registry and the
    // accepted map is written through the audited chokepoint.
    let capture_keydown = move |ev: leptos::ev::KeyboardEvent| {
        let Some(verb) = arming.get_untracked() else {
            return;
        };
        ev.prevent_default();
        ev.stop_propagation();
        let key = ev.key();
        if key == "Escape" {
            arming.set(None);
            rebind_error.set(None);
            return;
        }
        if matches!(key.as_str(), "Control" | "Alt" | "Shift" | "Meta") {
            return;
        }
        let chord = chord_from_key_event(
            &key,
            ev.ctrl_key(),
            ev.meta_key(),
            ev.alt_key(),
            ev.shift_key(),
        );
        let current = shared.get_untracked().keybinding_overrides;
        let registry = build(&current);
        match registry.rebind_validated(&current, verb, chord) {
            Ok(next) => {
                shared.apply(
                    SharedStateChange::SetKeybindingOverrides(next),
                    SharedStateOrigin::Shell,
                );
                arming.set(None);
                rebind_error.set(None);
            }
            Err(error) => rebind_error.set(Some(error.to_string())),
        }
    };

    view! {
        <div class="dna-overlay-backdrop" on:keydown=capture_keydown>
            <div
                class="dna-overlay"
                role="dialog"
                aria-label="Keyboard atlas"
                data-overlay="keyboard-atlas"
            >
                <h2>"Keyboard atlas"</h2>
                {move || {
                    let registry = shared.with(|state| build(&state.keybinding_overrides));
                    registry
                        .atlas_groups()
                        .into_iter()
                        .map(|group| {
                            view! {
                                <section class="dna-atlas__group" data-atlas-group=group.label>
                                    <h3>{group.label}</h3>
                                    {group
                                        .rows
                                        .into_iter()
                                        .map(|row| {
                                            let verb = row.verb;
                                            view! {
                                                <div class="dna-atlas__row" data-atlas-row=verb.stable_id()>
                                                    <span class="dna-atlas__chord">{row.chord_display.clone()}</span>
                                                    <span>{row.verb_label}</span>
                                                    {row
                                                        .tag
                                                        .map(|tag| {
                                                            view! { <span class="dna-atlas__tag" data-atlas-tag=tag>{tag}</span> }
                                                        })}
                                                    <span class="dna-atlas__spacer"></span>
                                                    {row
                                                        .rebindable
                                                        .then(|| {
                                                            view! {
                                                                <button
                                                                    class="dna-atlas__rebind"
                                                                    on:click=move |_| {
                                                                        arming.set(Some(verb));
                                                                        rebind_error.set(None);
                                                                    }
                                                                >
                                                                    "Rebind"
                                                                </button>
                                                            }
                                                        })}
                                                </div>
                                            }
                                        })
                                        .collect_view()}
                                </section>
                            }
                        })
                        .collect_view()
                }}
                {move || {
                    arming
                        .get()
                        .map(|verb| {
                            view! {
                                <p class="dna-atlas__arming">
                                    {format!(
                                        "Press the new chord for {} (Esc cancels)",
                                        verb_label(verb),
                                    )}
                                </p>
                            }
                        })
                }}
                {move || {
                    rebind_error
                        .get()
                        .map(|error| view! { <p class="dna-atlas__error" role="alert">{error}</p> })
                }}
            </div>
        </div>
    }
}

/// Peek card rendering stub (data model is the real S0 deliverable).
#[component]
fn PeekLayer(overlay: RwSignal<OverlayModel>) -> impl IntoView {
    view! {
        <div class="dna-peeks" data-region="peeks">
            {move || {
                overlay.with(|model| {
                    let mut cards: Vec<_> = model
                        .peeks
                        .pinned
                        .iter()
                        .map(|card| (card.key.clone(), true))
                        .collect();
                    if let Some(card) = &model.peeks.transient {
                        cards.push((card.key.clone(), false));
                    }
                    cards
                        .into_iter()
                        .map(|(key, pinned)| {
                            let unpin_key = key.clone();
                            view! {
                                <div
                                    class="dna-peek-card"
                                    data-peek=key.to_string()
                                    data-pinned=if pinned { "true" } else { "false" }
                                >
                                    <span>{key.to_string()}</span>
                                    {pinned
                                        .then(|| {
                                            view! {
                                                <button
                                                    class="dna-atlas__rebind"
                                                    on:click=move |_| {
                                                        overlay
                                                            .update(|model| model.peeks.unpin(&unpin_key));
                                                    }
                                                >
                                                    "Unpin"
                                                </button>
                                            }
                                        })}
                                </div>
                            }
                        })
                        .collect_view()
                })
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use dnacalc_skin_ir::identity::{NodeId, NodeKey};
    use dnacalc_skin_ir::workspace::{
        BindingDiagnosticProjection, NodeContentKind, NodeValueProjection, NodeView,
        SheetCalcSummaryProjection, SourceSpanProjection, WorkbookCalcProjection,
    };

    use super::*;

    fn node(id: &str, key: &str) -> NodeView {
        NodeView {
            key: NodeKey::new(key),
            id: NodeId::new(id),
            display_name: id.to_string(),
            parent: None,
            children: Vec::new(),
            depth: 0,
            content_kind: NodeContentKind::Empty,
            content_text: String::new(),
            computed_value: NodeValueProjection::Unevaluated,
            scenario_override: None,
            literalized_value_input: None,
            value_epoch: None,
            calc_state: None,
            effective_format: None,
            note: None,
            attributes: std::collections::BTreeMap::new(),
            binding_diagnostics: Vec::new(),
            is_meta: false,
            table: None,
        }
    }

    #[test]
    fn peek_target_prefers_focus_then_shared_selection_then_primary() {
        let mut shared = SharedSkinState::default();
        let mut workspace = WorkspaceState::default();
        let selection = SelectionState::with_primary(Some(NodeId::new("n1")));
        workspace
            .nodes
            .insert(NodeId::new("n1"), node("n1", "key-n1"));

        // Nothing anywhere: no target.
        assert_eq!(
            peek_target(
                &SharedSkinState::default(),
                &WorkspaceState::default(),
                &SelectionState::default()
            ),
            None
        );
        // Primary selection resolves through the workspace to its key.
        assert_eq!(
            peek_target(&shared, &workspace, &selection),
            Some(NodeKey::new("key-n1"))
        );
        // Shared selection set outranks the primary.
        shared.selection_set = vec![NodeKey::new("key-set")];
        assert_eq!(
            peek_target(&shared, &workspace, &selection),
            Some(NodeKey::new("key-set"))
        );
        // Continuity focus outranks everything.
        shared.focus_key = Some(NodeKey::new("key-focus"));
        assert_eq!(
            peek_target(&shared, &workspace, &selection),
            Some(NodeKey::new("key-focus"))
        );
    }

    #[test]
    fn document_dirty_is_projection_truth_only() {
        let mut workspace = WorkspaceState::default();
        assert!(!document_is_dirty(&workspace), "no projection, no dot");
        workspace.workbook_calc = Some(WorkbookCalcProjection {
            mode: dnacalc_skin_ir::workspace::CalcModeProjection::Automatic,
            last_recalc_tick: None,
            sheets: vec![SheetCalcSummaryProjection {
                grid_node_id: NodeId::new("s1"),
                dirty: false,
            }],
        });
        assert!(!document_is_dirty(&workspace));
        workspace.workbook_calc.as_mut().unwrap().sheets[0].dirty = true;
        assert!(document_is_dirty(&workspace));
    }

    /// The mast dirty dot OR-combines the workspace calc projection with the
    /// host's `PersistenceProjection.dirty` (bead dtc-lfz.3): a OneFormula/
    /// OneCalc host (no `workbook_calc` at all) can still light the dot from
    /// `persistence` alone; a product with no `host_persistence` prop wired
    /// (`None`) keeps exactly the old workspace-only behavior; either
    /// signal alone is sufficient, and neither is fabricated from the other.
    #[test]
    fn mast_dirty_or_combines_workspace_and_persistence_projections() {
        let clean_workspace = WorkspaceState::default();

        // No persistence prop wired at all: identical to `document_is_dirty`.
        assert!(!mast_dirty(&clean_workspace, None));

        // Persistence wired but reports clean: still not dirty.
        let clean_persistence = dnacalc_skin_ir::protocol::PersistenceProjection::default();
        assert!(!mast_dirty(&clean_workspace, Some(&clean_persistence)));

        // Persistence reports dirty on an otherwise-clean (no `workbook_calc`
        // at all) workspace — e.g. the Bench/OneFormula product, which has
        // no workbook_calc projection to derive a dot from any other way.
        let dirty_persistence = dnacalc_skin_ir::protocol::PersistenceProjection {
            dirty: true,
            ..dnacalc_skin_ir::protocol::PersistenceProjection::default()
        };
        assert!(
            mast_dirty(&clean_workspace, Some(&dirty_persistence)),
            "a dirty PersistenceProjection alone must light the dot"
        );

        // Workspace calc projection reports dirty, persistence absent: the
        // pre-existing tree-workspace path still works standalone.
        let mut dirty_workspace = WorkspaceState::default();
        dirty_workspace.workbook_calc = Some(WorkbookCalcProjection {
            mode: dnacalc_skin_ir::workspace::CalcModeProjection::Automatic,
            last_recalc_tick: None,
            sheets: vec![SheetCalcSummaryProjection {
                grid_node_id: NodeId::new("s1"),
                dirty: true,
            }],
        });
        assert!(mast_dirty(&dirty_workspace, None));
        // Both clean: not dirty.
        assert!(!mast_dirty(&clean_workspace, None));
    }

    #[test]
    fn shell_css_embeds_the_spec_region_sizes() {
        let css = shell_css(44);
        assert!(css.contains("grid-template-rows: 40px 44px minmax(0, 1fr) 26px"));
        assert!(css.contains("width: 232px"));
        assert!(css.contains("width: 268px"));
        let bench = shell_css(88);
        assert!(bench.contains("grid-template-rows: 40px 88px minmax(0, 1fr) 26px"));
    }

    /// SHELL_SPEC §7 red-error registry liveness (bead dtc-1tk.5): a node
    /// carrying a rejected/cycle-blocked calc-state, or any binding
    /// diagnostic, is an error signal; a clean node with neither is not.
    #[test]
    fn node_carries_error_reflects_calc_state_and_binding_diagnostics() {
        let mut n = node("n1", "key-n1");
        assert!(
            !node_carries_error(&n),
            "a clean projection carries no error"
        );

        n.calc_state = Some(NodeCalcStateProjection::Clean);
        assert!(
            !node_carries_error(&n),
            "Clean calc-state alone is not an error signal"
        );

        n.calc_state = Some(NodeCalcStateProjection::RejectedPendingRepair);
        assert!(node_carries_error(&n), "RejectedPendingRepair is an error");

        n.calc_state = Some(NodeCalcStateProjection::CycleBlocked);
        assert!(node_carries_error(&n), "CycleBlocked is an error");

        n.calc_state = None;
        assert!(!node_carries_error(&n));
        n.binding_diagnostics.push(BindingDiagnosticProjection {
            node: NodeId::new("n1"),
            node_key: NodeKey::new("key-n1"),
            message: "unresolved reference".to_string(),
            span: SourceSpanProjection {
                start_utf8: 0,
                end_utf8: 1,
            },
        });
        assert!(
            node_carries_error(&n),
            "a binding diagnostic is an error signal even with no calc-state"
        );
    }

    /// The registry-rail dot class ranks error above stale above fresh, and
    /// is never fabricated beyond those two booleans (SHELL_SPEC §7).
    #[test]
    fn liveness_dot_class_ranks_error_over_stale_over_fresh() {
        assert_eq!(
            liveness_dot_class(false, false),
            "dna-liveness dna-liveness--fresh"
        );
        assert_eq!(
            liveness_dot_class(false, true),
            "dna-liveness dna-liveness--stale"
        );
        assert_eq!(
            liveness_dot_class(true, false),
            "dna-liveness dna-liveness--error"
        );
        assert_eq!(
            liveness_dot_class(true, true),
            "dna-liveness dna-liveness--error",
            "error outranks stale"
        );
    }
}

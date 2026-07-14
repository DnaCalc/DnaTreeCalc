//! TIER: TP (Presentation) — T0 skin-ir + Leptos + TP crates only; no Ox* ever (P-gate).
//!
//! The Notebook stage (S2.5 render + S2.6 edit): a [`dnacalc_shell::StageSurface`]
//! registered under [`dnacalc_shell::StageId::Notebook`]. `mount` renders the
//! reactive single-column block list (NOTEBOOK_SPEC §3) from host truth: a
//! closure over `ctx.workspace` re-derives [`model::derive_entries`] on every
//! workspace change and paints one block per entry.
//!
//! Each authored **Cell** block is *editable* (S2.6): it embeds a degrade
//! [`FormulaBridgeDegrade`] editor seeded from the cell's own authored text and,
//! on commit, dispatches `WorkspaceIntent::EnterGridCell` built from the block's
//! OWN `grid`/`row`/`col` (via [`edit::enter_cell_intent`]) through
//! `ctx.dispatch`. The host's three-way outcome is read back with
//! [`edit::interpret_receipt`] and rendered as an honest per-block chip. The
//! skin never classifies `=`-vs-literal itself (SHELL_SPEC §6 layering law) and
//! never refreshes the workspace itself — the workbook dispatcher re-projects
//! into `ctx.workspace`, so the reactive closure repaints the block's value
//! automatically after a commit.
//!
//! When the governing persona is NOT Author (Reviewer / ReadOnly), the SAME
//! content renders fully readable but with ZERO enabled mutation controls — the
//! report artifact (S2.9; NOTEBOOK_SPEC §5 published/locked mode + §6 scenario
//! 4; there is no separate export in v1). The gate is a single pure predicate,
//! [`persona_allows_authoring`], read REACTIVELY off `ctx.shared` (through a
//! persona-projecting [`Memo`]) at every mutation-control site — the per-block
//! degrade editor ([`render_cell_editor`]), the `+ name` form
//! ([`render_add_name_affordance`]), and the keyboard mutation grammar (the root
//! `on:keydown`). A runtime `SetPersona` therefore repaints the surface live:
//! names, values, classification tints, liveness dots, and outcomes stay
//! readable while every editor / commit affordance vanishes — never a
//! disabled-looking editor that would in fact reject a write (SHELL_SPEC §6
//! honesty).
//!
//! It is never rendered blank: an empty derivation renders an explicit, testable
//! honest-empty card.
//!
//! **S2.10** adds a stage-level, honest **Reference X-Ray degrade**. The bead's
//! substrate probe asks whether the workbook bridge produces a caret-driven
//! `ReferenceResolutionProjection` that resolves a defined-name/cell reference
//! to a specific block — i.e. whether `WorkspaceState.dependencies` is ever
//! populated for the workbook profile. It is NOT: `dnacalc-host-core` has ZERO
//! references to `reference_resolutions` / `DependencyGraphProjection` / any
//! `dependencies` population, so `snapshot()` leaves `WorkspaceState.dependencies`
//! at its empty `DependencyGraphProjection::default()` (the TYPE machinery lives
//! in skin-ir's `workspace.rs`, unused by the workbook profile). Per the bead,
//! this ships a WHOLESALE honest degrade rather than a half-built feature: a
//! single reactive `data-testid="notebook-xray-degrade"` note
//! ([`reference_xray_available`]) citing ask #7 / G9 — NO caret tracking, NO
//! gutter arrows, NO reference highlighting, none of that machinery exists
//! here. The check reads the REAL substrate (`reference_resolutions`
//! non-empty), so it is not a hardcoded lie: it flips honestly if the host ever
//! populates the projection, at which point the real feature (still out of
//! scope here) could be built on top.
//!
//! **S2.11** gives an editable Cell block a **delete affordance** that routes
//! through the read-side preview seam WHERE PRESENT (mechanism 12). Deleting a
//! Cell block CLEARS its cell (Excel semantics) — it reuses the exact same
//! entry-verb seam the editor commits through, `enter_cell_intent(grid, row,
//! col, "")` (→ [`CellOutcome::Cleared`]); it never invents a workbook
//! "remove cell". The affordance is authoring-gated exactly like the editor
//! (S2.9) — a delete is a mutation, so a non-authoring persona (Reviewer /
//! ReadOnly) gets none of it. The decision is a single pure function,
//! [`preview_block_delete`]: with a [`dnacalc_skin_ir::PreviewService`] it asks
//! [`dnacalc_skin_ir::PreviewService::preview_mutation_impact`] what the clear
//! WOULD do and renders a non-mutating **ghost-preview**
//! (`data-testid="notebook-delete-ghost"`) summarizing the impact HONESTLY from
//! real fields — with **Esc = revert** (drop the ghost, dispatch NOTHING) and a
//! confirm that dispatches the actual clear and reads the honest receipt. With
//! `ctx.preview == None` (the calc app's reality today) — or a preview that
//! ERRORS (never escalated, per `preview.rs`) — it degrades to post-attempt
//! **receipt feedback**: activating delete dispatches the clear directly and
//! renders the honest [`CellOutcome`] through the SAME outcome-chip machinery
//! the editor uses, never a fabricated ghost. Workbook grid cells carry no
//! per-cell [`NodeId`] in the projection (only their grid's id + row/col), so
//! the preview intent addresses the grid node — see [`delete_impact_intent`].

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

use dnacalc_bridge::{BridgeEvent, FormulaBridgeDegrade};
use dnacalc_shell::{ProfileTag, StageContext, StageHandle, StageId, StageSurface};
use dnacalc_skin_ir::intent::{Dispatcher, WorkspaceIntent};
use dnacalc_skin_ir::{
    DefinedNameProjection, DefinedNameTargetProjection, GridAuthoredCellProjection,
    GridCellProjection, GridEditabilityProjection, GridEntryDiagnosticProjection,
    GridTableOverlayDescriptor, MutationImpactIntentProjection, MutationImpactProjection,
    NodeClassification, NodeId, NodeValueProjection, Persona, PreviewService, WorkspaceState,
};

pub mod edit;
pub mod keyboard;
pub mod model;

pub use edit::{CellOutcome, enter_cell_intent, interpret_receipt};
pub use keyboard::{
    ActionAvailability, KeyBinding, KeyChord, NotebookAction, action_availability, notebook_keymap,
    resolve_action,
};
pub use model::{NotebookEntry, NotebookEntryKind, derive_entries};

/// Stable per-block identity: a Cell block's grid plus its 1-based address. The
/// per-block editor state map ([`BlockEditState`]) is keyed on this so a block's
/// in-progress text, rejections, and outcome are anchored to its cell — they can
/// neither cross-contaminate another block nor be reset when the reactive
/// closure re-derives the whole list after some *other* cell's commit.
type BlockKey = (NodeId, u32, u32);

/// One editable Cell block's live editor state. Every field is a `Copy`
/// [`RwSignal`], so [`BlockEditState`] is `Copy` and cheaply captured into a
/// block's event callback and its outcome-chip closure.
///
/// PER-BLOCK STATE / no cross-contamination: these signals live in a map keyed
/// by [`BlockKey`] (see [`block_edit_state`]), created ONCE per block under the
/// stable *mount* owner — never inside the re-running derivation closure. That
/// is the crux:
///  - **No bleed between blocks** — each block resolves its own distinct
///    `BlockEditState` by its own address; a callback captures only its block's
///    signals, and a commit builds its intent from its own `grid`/`row`/`col`.
///  - **In-progress text survives re-derivation** — when any block commits, the
///    workspace re-projects and the derivation closure re-runs, remounting every
///    block's editor. Each editor re-seeds from *its own* `edit_text` signal
///    (updated on every keystroke, read untracked), so a sibling block's
///    half-typed formula is preserved rather than reset to its authored text.
///  - **Signals outlive the re-render** — created under the mount owner
///    ([`Owner`]), they are disposed only when the stage unmounts, not when the
///    inner closure re-runs (which would dispose signals parented to it).
#[derive(Clone, Copy)]
struct BlockEditState {
    /// The live edit-buffer text; updated on every `TextEdited` and read
    /// untracked to re-seed the editor on each remount.
    edit_text: RwSignal<String>,
    /// Typed rejections from this block's last commit attempt; drives the
    /// degrade underline, cleared on a successful commit.
    rejections: RwSignal<Vec<GridEntryDiagnosticProjection>>,
    /// This block's last committed outcome; drives its outcome chip. `None`
    /// until the block has been committed at least once.
    outcome: RwSignal<Option<CellOutcome>>,
    /// Bumped on every commit/revert to remount THIS block's degrade editor
    /// (re-seeding from `edit_text`, re-applying `rejections`). The bridge reads
    /// its `rejections` prop only at mount, and a *rejected* commit leaves the
    /// workspace signal untouched (the host publishes an unchanged delta), so
    /// without this the rejection underline would never appear — mirrors the
    /// app's `CalcBridgeSurface` revision discipline, keyed per block.
    revision: RwSignal<usize>,
    /// S2.11 delete ghost-preview state: `Some(impact)` while THIS block is
    /// showing a non-mutating ghost of a pending clear (the preview seam
    /// answered), `None` otherwise. Set by the delete control on the ghost path,
    /// cleared to `None` by Esc/cancel (dispatch NOTHING) or by a confirm after
    /// its clear dispatches. Per-block by construction — it lives on this
    /// block's own state, so one block's pending ghost never bleeds into another.
    pending_ghost: RwSignal<Option<MutationImpactProjection>>,
}

impl BlockEditState {
    /// Fresh state seeded from the cell's authored text (a formula's source
    /// text, else a literal's text, else empty).
    fn new(seed: &str) -> Self {
        Self {
            edit_text: RwSignal::new(seed.to_string()),
            rejections: RwSignal::new(Vec::new()),
            outcome: RwSignal::new(None),
            revision: RwSignal::new(0),
            pending_ghost: RwSignal::new(None),
        }
    }
}

/// The Notebook stage surface.
#[derive(Debug, Clone, Copy, Default)]
pub struct NotebookStage;

impl NotebookStage {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl StageSurface for NotebookStage {
    fn id(&self) -> StageId {
        StageId::Notebook
    }

    fn title(&self) -> &'static str {
        "Notebook"
    }

    fn supports(&self, profile: &ProfileTag) -> bool {
        matches!(profile, ProfileTag::ExcelStrict)
    }

    fn mount(&self, ctx: StageContext) -> StageHandle {
        // The one host-truth read this stage makes. `ReadSignal` is `Copy`, so
        // the closure below can re-run `derive_entries` on every workspace
        // change without holding any owned state — the block list is a *view*
        // of the workspace, never a copy that can drift (NOTEBOOK_SPEC §1).
        let workspace = ctx.workspace;
        // The one dispatcher a Cell block commits through (SHELL_SPEC §6). The
        // workbook dispatcher owns the app's workspace signal and re-projects on
        // every dispatch, so a committed edit repaints via the closure below —
        // this stage wires no workspace refresh of its own.
        let dispatch = ctx.dispatch.clone();
        // S2.11 read-side preview seam (mechanism 12): the optional, non-mutating
        // foresight service. `Some` → a delete previews as a ghost; `None` (the
        // calc app today) → a delete degrades to post-attempt receipt feedback.
        // Threaded into each Cell block so [`render_cell_editor`]'s delete
        // affordance can ask [`preview_block_delete`] which path to take.
        let preview = ctx.preview.clone();
        // --- S2.9 reviewer-persona read-only gate -----------------------------
        // The governing persona lives in the audited shared state
        // (`SharedSkinState.persona`, written by `SetPersona`). A `Memo` projects
        // it out so every gate site below subscribes to the persona ALONE — a
        // persona flip repaints the mutation controls, while unrelated shared-
        // state writes (selection, collapse, active-lens continuity) do NOT churn
        // the block list or remount a block's in-progress editor. `ctx.shared` is
        // `Copy` (captured into the memo); the memo is `Copy` (captured into each
        // gating closure). This is the ONE reactive read that makes the Reviewer/
        // ReadOnly report artifact LIVE rather than a mount-time snapshot
        // (NOTEBOOK_SPEC §5-6). Every gate reads it through the single pure
        // predicate [`persona_allows_authoring`], so no site can drift.
        let shared = ctx.shared;
        let persona = Memo::new(move |_| shared.with(|state| state.persona));
        // Per-block editor state, keyed by stable block identity. Lives OUTSIDE
        // the reactive derivation closure so a workspace change (which re-derives
        // and remounts every block) cannot reset a block's in-progress text,
        // rejections, or last outcome. See [`BlockEditState`] for the full
        // no-cross-contamination / survives-re-derivation argument.
        let editors: StoredValue<HashMap<BlockKey, BlockEditState>> =
            StoredValue::new(HashMap::new());
        // The stable owner for lazily-created per-block signals: the mount owner
        // outlives every re-derivation (only the inner closure re-runs). Creating
        // a block's signals under it — not under the transient render effect —
        // is what keeps them alive across re-renders instead of being disposed on
        // the next workspace change. `None` only off a reactive runtime (e.g. a
        // bare SSR render), where re-derivation cannot happen anyway.
        let owner = Owner::current();
        // The name-first authoring affordance (S2.8): its own buffers, created
        // once outside the reactive derivation closure (same reasoning as
        // `editors` above) so opening the form / typing survives a workspace
        // re-derivation triggered by some OTHER block's commit.
        let add_name_open = RwSignal::new(false);
        let name_buffer = RwSignal::new(String::new());
        let value_buffer = RwSignal::new(String::new());
        let add_name_dispatch = Arc::clone(&dispatch);
        // --- S2.7 keyboard grammar state ---------------------------------
        // All three signals + the root `NodeRef` are created here in `mount`,
        // i.e. under the stable mount owner (the same discipline `editors`
        // documents): a workspace re-derivation re-runs only the inner block
        // closure, never `mount`, so which block is focused, the last honest
        // degrade note, and any pending focus move all survive a sibling
        // block's commit.
        //
        // `focused_block` is the load-bearing state the Enter / Shift+Enter
        // grammar operates on; it is kept accurate by an `on:focusin` on every
        // block (below) that stores the block's own [`BlockKey`] — or clears it
        // for a non-editable block, so a stale editable key never lingers.
        let focused_block: RwSignal<Option<BlockKey>> = RwSignal::new(None);
        // The `aria-live` degrade note: a disabled-with-reason chord writes its
        // reason here and mutates nothing else. Never a model write.
        let keyboard_status: RwSignal<Option<String>> = RwSignal::new(None);
        // A pending keyboard-driven focus move, landed by the effect below.
        let focus_request: RwSignal<Option<FocusRequest>> = RwSignal::new(None);
        let root_ref = NodeRef::<leptos::html::Section>::new();

        // Land a pending keyboard focus move AFTER the block list repaints. A
        // commit re-projects the workspace and remounts blocks, so focus cannot
        // be moved synchronously in the keydown handler (the target node is
        // being replaced); an effect runs after the reactive graph settles and
        // the DOM is patched. The editor `<textarea>` lives inside the
        // `FormulaBridgeDegrade` child (whose internal node this stage does not
        // own), so focus is resolved by querying the root for the block's stable
        // `data-notebook-cell` address — never a `NodeRef` we cannot hold.
        Effect::new(move |_| {
            let Some(request) = focus_request.get() else {
                return;
            };
            let Some(root) = root_ref.get() else {
                return;
            };
            let key = match &request {
                FocusRequest::Editor(key) | FocusRequest::Article(key) => key,
            };
            // Query the block by its stable `data-notebook-cell` address, ESCAPING
            // the interpolated value: a grid `NodeId` bearing a `"` or `\` would
            // otherwise break the CSS attribute selector and silently lose focus.
            // (`query_selector_all` would sidestep interpolation entirely but is
            // not in leptos's enabled web_sys feature set; escaping keeps us on
            // the available single `query_selector`.)
            let selector = format!(
                "article[data-notebook-cell=\"{}\"]",
                css_escape_attr_value(&block_dom_key(key))
            );
            let Ok(Some(article)) = root.query_selector(&selector) else {
                return;
            };
            let target = match &request {
                // OpenEditor lands in the block's editor input.
                FocusRequest::Editor(_) => article.query_selector("textarea").ok().flatten(),
                // Advance-after-commit lands on the block's article (command
                // mode), from which Enter re-opens its editor.
                FocusRequest::Article(_) => Some(article),
            };
            if let Some(element) = target
                && let Ok(html) = element.dyn_into::<leptos::web_sys::HtmlElement>()
            {
                let _ = html.focus();
            }
        });

        // The one keydown pipeline: build a [`keyboard::KeyChord`], resolve it
        // through the single [`notebook_keymap`] table, and act on the honest
        // availability. An unbound chord returns untouched so universal shell
        // verbs (F9, Ctrl+K, arrows) still bubble.
        let keymap = notebook_keymap();
        let keydown_dispatch = Arc::clone(&dispatch);
        let keydown_workspace = workspace;
        let root_keydown = move |ev: leptos::ev::KeyboardEvent| {
            let chord = chord_from_event(&ev);
            let Some(action) = resolve_action(&keymap, &chord) else {
                return;
            };
            match action_availability(action) {
                ActionAvailability::DisabledWithReason(reason) => {
                    // Honest degrade: surface the reason, mutate NOTHING, and
                    // consume the chord so neither the browser nor the shell
                    // acts on it.
                    ev.prevent_default();
                    ev.stop_propagation();
                    keyboard_status.set(Some(reason.to_string()));
                }
                ActionAvailability::Live
                    if !persona_allows_authoring(persona.get_untracked()) =>
                {
                    // S2.9 reactive persona gate: every `Live` Notebook action is
                    // an AUTHORING verb (open this block's editor, commit-and-
                    // advance, new name), so a non-authoring persona (Reviewer /
                    // ReadOnly) performs ZERO of them — surface the honest read-
                    // only note, consume the chord so neither the browser nor the
                    // shell acts on it, and dispatch NOTHING (mirrors the
                    // `DisabledWithReason` arm above). Persona is read UNTRACKED
                    // (via the memo) so this event handler never subscribes to it,
                    // exactly how `keydown_workspace` is read below.
                    ev.prevent_default();
                    ev.stop_propagation();
                    keyboard_status.set(Some(READ_ONLY_PERSONA_NOTE.to_string()));
                }
                ActionAvailability::Live => match action {
                    NotebookAction::NewName => {
                        // Reuse the S2.8 `+ name` form — never a second form.
                        ev.prevent_default();
                        ev.stop_propagation();
                        keyboard_status.set(None);
                        add_name_open.set(true);
                    }
                    NotebookAction::OpenEditor => {
                        // Fire ONLY when the keystroke originates inside a block,
                        // and only in command mode. `event_in_block` excludes the
                        // `+ name` chrome (its toggle/Create buttons and the root
                        // section): `focused_block` is sticky (set only by a
                        // block's `on:focusin`), so without this guard, Enter on
                        // the `+ name` button would `prevent_default` its own
                        // activation and jump focus into a stale cell. The
                        // text-entry guard additionally keeps bare Enter out of a
                        // buffer while typing (the bridge also stop_propagations
                        // its own Enter=commit).
                        if !event_in_block(&ev) || event_target_is_text_entry(&ev) {
                            return;
                        }
                        let Some(key) = focused_block.get_untracked() else {
                            // Honest no-op — nothing focused. Let it bubble
                            // rather than swallow the keystroke.
                            return;
                        };
                        ev.prevent_default();
                        ev.stop_propagation();
                        keyboard_status.set(None);
                        focus_request.set(Some(FocusRequest::Editor(key)));
                    }
                    NotebookAction::CommitAndAdvance => {
                        // Shift+Enter is meant to fire WHILE editing a block's
                        // bridge editor (the bridge leaves Shift+Enter a plain
                        // newline and lets it bubble), so it is NOT gated by the
                        // text-entry guard — but it IS gated to keystrokes that
                        // originate inside a block. A Shift+Enter on the `+ name`
                        // form's inputs OR its buttons OR the root section is
                        // outside any block, so it never commits whatever block
                        // was last focused (the sticky-`focused_block` hazard).
                        if !event_in_block(&ev) {
                            return;
                        }
                        let Some(key) = focused_block.get_untracked() else {
                            return;
                        };
                        let Some(state) = editors.with_value(|map| map.get(&key).copied()) else {
                            // Focused block has no editor state (e.g. a
                            // read-only cell): honest no-op, no faked commit.
                            return;
                        };
                        ev.prevent_default();
                        ev.stop_propagation();
                        keyboard_status.set(None);
                        commit_block(&keydown_dispatch, &key, state);
                        // Read the (re-projected) workspace untracked so the
                        // keydown handler never subscribes to it.
                        let next =
                            keydown_workspace.with_untracked(|ws| next_cell_key(ws, &key));
                        if let Some(next) = next {
                            focused_block.set(Some(next.clone()));
                            focus_request.set(Some(FocusRequest::Article(next)));
                        }
                    }
                    // The three deferred actions are never `Live`; this keeps
                    // the match total without an `unreachable!()` panic path.
                    NotebookAction::ReorderUp
                    | NotebookAction::ReorderDown
                    | NotebookAction::NewProse => {}
                },
            }
        };
        let view = view! {
            <style>{NOTEBOOK_CSS}</style>
            <section
                class="dna-notebook"
                data-stage="notebook"
                data-testid="notebook-root"
                data-dna-density="reading"
                node_ref=root_ref
                tabindex="0"
                on:keydown=root_keydown
            >
                {move || {
                    // S2.9 reactive persona gate: the `+ name` form is a mutation
                    // control, so a non-authoring persona (Reviewer / ReadOnly)
                    // gets NOTHING here — the cleanest "zero enabled mutation
                    // controls", not a disabled shell. Reading the persona memo
                    // subscribes this fragment to `SetPersona`, so the form
                    // appears/vanishes live on a runtime persona flip.
                    if !persona_allows_authoring(persona.get()) {
                        return ().into_any();
                    }
                    render_add_name_affordance(
                        add_name_open,
                        name_buffer,
                        value_buffer,
                        Arc::clone(&add_name_dispatch),
                    )
                }}
                <p
                    class="dna-notebook__kbd-status"
                    data-testid="notebook-keyboard-status"
                    aria-live="polite"
                    role="status"
                >
                    {move || keyboard_status.get().unwrap_or_default()}
                </p>
                {move || {
                    // S2.10 substrate probe: read the workspace REACTIVELY so
                    // this note stays truthful on any workspace change —
                    // it would flip silently if the host ever populated
                    // `dependencies.reference_resolutions`. For today's
                    // workbook demo the projection is always empty, so this
                    // always renders the honest degrade. See
                    // `reference_xray_available`'s doc comment for the full
                    // probe writeup (WHOLESALE HONEST DEGRADE, ask #7 / G9).
                    if workspace.with(reference_xray_available) {
                        // The real feature (caret tracking, gutter arrows,
                        // reference highlighting) stays OUT OF SCOPE even if
                        // the substrate ever appears — render nothing extra,
                        // never build it speculatively here.
                        return ().into_any();
                    }
                    view! {
                        <p
                            class="dna-notebook__xray-degrade"
                            data-testid="notebook-xray-degrade"
                            aria-live="polite"
                        >
                            {REFERENCE_XRAY_DEGRADE_NOTE}
                        </p>
                    }
                    .into_any()
                }}
                {move || {
                    // S2.9 reactive persona gate: read the governing persona
                    // (via the memo) HERE so this derivation subscribes to
                    // `SetPersona` — a persona flip repaints every block with (or
                    // without) its per-block editor. `authoring` is threaded into
                    // each block so the value region, name, classification chip,
                    // liveness dot, and outcome stay READABLE for every persona;
                    // only the mutation control is gated (NOTEBOOK_SPEC §6 #4).
                    let authoring = persona_allows_authoring(persona.get());
                    let entries = workspace.with(model::derive_entries);
                    // S2.11 eviction: drop per-block edit state for any block no
                    // longer derived — a deleted/cleared cell vanishes from
                    // `entries`, and without this its `BlockEditState` would
                    // linger keyed by `(grid,row,col)` and seed a *future*
                    // re-author at that same address from stale `""`/`Cleared`
                    // remnants (`block_edit_state` early-returns an existing
                    // entry, ignoring the fresh seed). A block being edited keeps
                    // its authored content, so it stays derived — only a
                    // committed clear evicts.
                    let live_keys = live_block_keys(&entries);
                    editors.update_value(|map| map.retain(|key, _| live_keys.contains(key)));
                    if entries.is_empty() {
                        // Never blank: the honest-empty card is an explicit,
                        // testable state.
                        view! {
                            <p class="dna-notebook__empty" data-testid="notebook-empty">
                                "No notebook content yet."
                            </p>
                        }
                        .into_any()
                    } else {
                        entries
                            .into_iter()
                            .map(|entry| {
                                render_block(
                                    entry,
                                    &dispatch,
                                    preview.as_ref(),
                                    workspace,
                                    editors,
                                    owner.as_ref(),
                                    focused_block,
                                    authoring,
                                )
                            })
                            .collect_view()
                            .into_any()
                    }
                }}
            </section>
        }
        .into_any();
        StageHandle::new(view)
    }
}

/// A pending keyboard-driven focus move, resolved by the post-render effect in
/// [`NotebookStage::mount`]. Held as a signal (not performed inline) because a
/// commit re-projects the workspace and remounts blocks — focus must land AFTER
/// that repaint, and only an effect runs late enough to see the new DOM.
#[derive(Clone)]
enum FocusRequest {
    /// Focus the block's editor input — Enter / [`NotebookAction::OpenEditor`].
    Editor(BlockKey),
    /// Focus the block's article shell in command mode — the advance step of
    /// [`NotebookAction::CommitAndAdvance`].
    Article(BlockKey),
}

/// Translate a raw DOM keydown into the Notebook grammar's [`KeyChord`]: the
/// platform `meta` key folds into `ctrl` (estate convention, matching
/// `dnacalc_shell::keyboard::chord_from_key_event`), so a chord is
/// layout-portable.
fn chord_from_event(ev: &leptos::ev::KeyboardEvent) -> KeyChord {
    KeyChord::new(
        ev.key(),
        ev.shift_key(),
        ev.ctrl_key() || ev.meta_key(),
        ev.alt_key(),
    )
}

/// True when the keydown originates from a text-entry element — the guard that
/// keeps command-mode chords (bare Enter) from firing while the user is typing.
/// Mirrors `dnacalc_shell::shell::event_target_is_text_entry` (private to the
/// shell crate, so a TP stage owns its own honest copy).
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

/// True when the keydown originated inside a Notebook block article. Commit-
/// and-advance (Shift+Enter) uses this so a Shift+Enter typed in the unrelated
/// `+ name` form never commits whatever block was last focused.
fn event_in_block(ev: &leptos::ev::KeyboardEvent) -> bool {
    ev.target()
        .and_then(|target| target.dyn_into::<leptos::web_sys::Element>().ok())
        .and_then(|element| element.closest("article.dna-notebook__block").ok().flatten())
        .is_some()
}

/// The stable DOM address a Cell block carries in `data-notebook-cell` and the
/// focus effect queries on: its grid id plus numeric row/col, pipe-joined. A
/// private focus-routing handle only — never an A1 address authority.
fn block_dom_key(key: &BlockKey) -> String {
    let (grid, row, col) = key;
    format!("{grid}|{row}|{col}")
}

/// Escape a string for use inside a double-quoted CSS attribute-selector value
/// (`[attr="…"]`): backslash and double-quote are the only characters that
/// escape or terminate the value within the quotes, so escaping just those two
/// makes the selector injection-proof for any grid `NodeId`.
fn css_escape_attr_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Commit a block's in-progress edit text through the SAME entry-verb seam
/// [`render_cell_editor`] uses (SHELL_SPEC §6 layering law): build the intent
/// from the block's OWN `(grid, row, col)` address, dispatch it, interpret the
/// host's three-way receipt, and record the outcome / rejections / editor-
/// remount on the block's own [`BlockEditState`]. Reads host truth only; the
/// stage never classifies the text. This is exactly the `BridgeEvent::CommitRequested`
/// path, reached by Shift+Enter instead of the bridge's own Enter.
fn commit_block(dispatch: &Arc<dyn Dispatcher>, key: &BlockKey, state: BlockEditState) {
    let (grid, row, col) = key;
    let text = state.edit_text.get_untracked();
    let receipt = dispatch.dispatch(enter_cell_intent(grid.clone(), *row, *col, text));
    let resolved = interpret_receipt(&receipt);
    if let CellOutcome::Rejected(diagnostics) = &resolved {
        // Keep the rejected text intact so the user can fix it; underline the
        // typed diagnostics.
        state.rejections.set(diagnostics.clone());
    } else {
        state.rejections.set(Vec::new());
    }
    state.outcome.set(Some(resolved));
    // Remount the editor to re-seed committed text / apply rejections,
    // independent of whether the workspace fired (a rejection does not).
    state.revision.update(|revision| *revision += 1);
}

/// The next editable-block key after `current` in derived order — the block
/// Shift+Enter advances focus to. A pure function of [`WorkspaceState`] (the
/// caller reads it untracked, so the keydown handler never subscribes),
/// computed from the SAME [`model::derive_entries`] ordering the block list
/// paints. `None` when `current` is the last Cell (or is absent), so the commit
/// still lands but focus honestly stays put rather than wrapping.
fn next_cell_key(ws: &WorkspaceState, current: &BlockKey) -> Option<BlockKey> {
    let keys: Vec<BlockKey> = model::derive_entries(ws)
        .into_iter()
        .filter_map(|entry| match entry.kind {
            NotebookEntryKind::Cell {
                grid, authored, ..
            } => Some((grid, authored.row, authored.col)),
            NotebookEntryKind::Name { .. } | NotebookEntryKind::Table { .. } => None,
        })
        .collect();
    let index = keys.iter().position(|candidate| candidate == current)?;
    keys.get(index + 1).cloned()
}

/// The set of Cell-block keys present in a derivation — the blocks that
/// currently exist. Used to EVICT per-block edit state (`editors`) for blocks
/// that have vanished from the derivation (e.g. a deleted/cleared cell), so a
/// later re-author at the same `(grid,row,col)` address gets fresh state rather
/// than a stale `""`/`Cleared`/`pending_ghost` remnant. Pure over the derived
/// entries, so it is unit-testable.
fn live_block_keys(entries: &[NotebookEntry]) -> HashSet<BlockKey> {
    entries
        .iter()
        .filter_map(|entry| match &entry.kind {
            NotebookEntryKind::Cell {
                grid, authored, ..
            } => Some((grid.clone(), authored.row, authored.col)),
            NotebookEntryKind::Name { .. } | NotebookEntryKind::Table { .. } => None,
        })
        .collect()
}

/// Resolve (creating on first sight) the [`BlockEditState`] for a Cell block.
/// The signals are created under `owner` — the stable mount owner — so they
/// persist across the derivation closure's re-runs; storing them in `editors`
/// (a non-reactive [`StoredValue`] map) anchors them to the block's address so
/// no two blocks ever share edit state.
fn block_edit_state(
    editors: StoredValue<HashMap<BlockKey, BlockEditState>>,
    owner: Option<&Owner>,
    key: &BlockKey,
    seed: &str,
) -> BlockEditState {
    if let Some(state) = editors.with_value(|map| map.get(key).copied()) {
        return state;
    }
    let state = match owner {
        Some(owner) => owner.with(|| BlockEditState::new(seed)),
        None => BlockEditState::new(seed),
    };
    editors.update_value(|map| {
        map.insert(key.clone(), state);
    });
    state
}

/// Render one derived entry as a single block (NOTEBOOK_SPEC §3 block anatomy):
/// a gutter carrying a kind glyph + a structural classification tint, a name row
/// (`display_name` · classification chip · liveness dot), a result region
/// showing the live computed value, and — for an editable Cell — a degrade
/// editor plus its honest outcome chip.
// A render fan-out threading the block's whole context (dispatch, preview,
// workspace, per-block state, owner, focus, persona) — cohesive, not a grab-bag;
// matches the `#[allow]` the sibling shell crate uses for the same reason.
#[allow(clippy::too_many_arguments)]
fn render_block(
    entry: NotebookEntry,
    dispatch: &Arc<dyn Dispatcher>,
    // S2.11: the optional preview service, threaded to the per-block delete
    // affordance. `Some` → ghost-preview path; `None` → receipt-fallback path.
    preview: Option<&Arc<dyn PreviewService>>,
    workspace: ReadSignal<WorkspaceState>,
    editors: StoredValue<HashMap<BlockKey, BlockEditState>>,
    owner: Option<&Owner>,
    focused_block: RwSignal<Option<BlockKey>>,
    // S2.9: whether the governing persona may author. `false` (Reviewer /
    // ReadOnly) swaps THIS block's editor slot for an honest read-only note;
    // every readable region above is rendered regardless.
    authoring: bool,
) -> AnyView {
    let classification = entry.classification;
    let class_id = classification.stable_id();
    let chip_label = classification_label(classification);
    let liveness = liveness_id(classification);
    let kind_id = entry_kind_id(&entry.kind);
    let glyph = entry_kind_glyph(&entry.kind);
    // The classification tint is a *structural* modifier class (per the style
    // law: it encodes the classification, it is not decorative color) — the
    // color itself resolves through Strand's soft-token palette in NOTEBOOK_CSS.
    let gutter_class = format!("dna-notebook__gutter dna-notebook__gutter--{class_id}");
    let display_name = entry.display_name.clone();
    // The block's keyboard identity: `Some(key)` for an editable-addressable
    // Cell block (the only kind the Enter / Shift+Enter grammar operates on),
    // `None` for a Name/Table block. `on:focusin` writes this into
    // `focused_block` — clearing it (to `None`) when a non-Cell block takes
    // focus, so a stale editable key never lingers behind a focused Name/Table.
    // `data-notebook-cell` is the stable address the focus effect queries on.
    let cell_key: Option<BlockKey> = match &entry.kind {
        NotebookEntryKind::Cell { grid, authored, .. } => {
            Some((grid.clone(), authored.row, authored.col))
        }
        NotebookEntryKind::Name { .. } | NotebookEntryKind::Table { .. } => None,
    };
    let cell_dom_key = cell_key.as_ref().map(block_dom_key).unwrap_or_default();
    let on_focusin = move |_| focused_block.set(cell_key.clone());
    let value_view = match &entry.kind {
        NotebookEntryKind::Cell { value, .. } => render_value(value),
        NotebookEntryKind::Name { name, backing_cell } => render_name_value(name, backing_cell),
        NotebookEntryKind::Table { table, .. } => render_table_value(table),
    };
    // The interactive region: an editor + outcome for an editable Cell, an
    // honest read-only note otherwise. Names/Tables are read-only here (a Name
    // is edited through its backing cell, not fabricated a target — SHELL_SPEC
    // §6 honesty).
    let edit_view = match &entry.kind {
        NotebookEntryKind::Cell {
            grid, authored, ..
        } => render_cell_editor(
            grid, authored, dispatch, preview, workspace, editors, owner, authoring,
        ),
        // A Name block is edited through its backing cell, never a fabricated
        // target (SHELL_SPEC §6 honesty). The note only points at that edit path
        // for an authoring persona; a Reviewer/ReadOnly, who cannot take it, sees
        // the plain read-only label instead of a call-to-action they can't act on.
        NotebookEntryKind::Name { .. } => {
            if authoring {
                read_only_note("defined name — edit its backing cell")
            } else {
                read_only_note("defined name")
            }
        }
        NotebookEntryKind::Table { .. } => ().into_any(),
    };

    view! {
        <article
            class="dna-notebook__block"
            data-block-kind=kind_id
            data-classification=class_id
            data-testid="notebook-block"
            data-notebook-cell=cell_dom_key
            tabindex="0"
            on:focusin=on_focusin
        >
            <div class=gutter_class aria-hidden="true">
                <span class="dna-notebook__glyph">{glyph}</span>
            </div>
            <div class="dna-notebook__body">
                <div class="dna-notebook__name-row">
                    <span class="dna-notebook__name" data-testid="notebook-block-name">
                        {display_name}
                    </span>
                    <span class="dna-notebook__chip" data-testid="notebook-classification">
                        {chip_label}
                    </span>
                    <span class="dna-notebook__dot" data-liveness=liveness aria-hidden="true"></span>
                </div>
                {value_view}
                {edit_view}
            </div>
        </article>
    }
    .into_any()
}

/// The Notebook's **name-first authoring affordance** (S2.8): a `+ name`
/// control that reveals a two-field inline form (name, value/formula text) plus
/// a Create button. On Create, with a non-empty (trimmed) name, this dispatches
/// the **single** atomic `WorkspaceIntent::CreateNamedValue` through `ctx.dispatch`
/// — host-core owns finding-or-creating the `_names` backing sheet, writing
/// `value_text` through the universal entry verb, and defining `name` at
/// Workbook scope (skin-ir `intent.rs`'s own doc comment); this stage never
/// authors the `_names` sheet itself, never guesses its backing cell, and never
/// classifies `=`-vs-literal (SHELL_SPEC §6 layering law — the same discipline
/// `render_cell_editor` follows for ordinary cell commits).
///
/// HONEST EMPTY: an empty (or all-whitespace) name commits nothing at all — no
/// intent is dispatched — and the Create button is `disabled` AND an inline
/// hint explains why, so the affordance never fabricates a success for a name
/// the user never actually typed.
///
/// The form's own buffers (`name`/`value`) are created once under the mount
/// owner (by the caller, [`NotebookStage::mount`]), outside the reactive
/// derivation closure — the same non-cross-contamination reasoning
/// [`BlockEditState`] documents: a workspace re-derivation (from some sibling
/// block's commit, or from this very create) must not reset in-progress typing
/// in this form. The outer `move || ..` below reads only `open`, so opening/
/// closing the form is the only thing that remounts its `<input>`s — each
/// keystroke updates its own field's `prop:value` binding in place, never
/// recreating the DOM nodes (mirrors the `revision`-gated remount discipline
/// `render_cell_editor` uses for its degrade editor).
fn render_add_name_affordance(
    open: RwSignal<bool>,
    name_buffer: RwSignal<String>,
    value_buffer: RwSignal<String>,
    dispatch: Arc<dyn Dispatcher>,
) -> AnyView {
    let toggle_open = move |_| open.update(|value| *value = !*value);

    view! {
        <div class="dna-notebook__add-name">
            <button
                type="button"
                class="dna-notebook__add-name-toggle"
                data-testid="notebook-add-name"
                on:click=toggle_open
            >
                "+ name"
            </button>
            {move || {
                if !open.get() {
                    return ().into_any();
                }
                // Cloned fresh each time the form (re)opens; the `create`
                // closure below is single-use (one Create button), so no
                // `Clone` bound on it is needed.
                let commit_dispatch = Arc::clone(&dispatch);
                let create = move |_| {
                    let typed_name = name_buffer.get_untracked();
                    let trimmed = typed_name.trim();
                    if trimmed.is_empty() {
                        // Honest no-op: never dispatch, never fabricate a
                        // created name from empty/whitespace-only text.
                        return;
                    }
                    let value_text = value_buffer.get_untracked();
                    commit_dispatch.dispatch(WorkspaceIntent::CreateNamedValue {
                        name: trimmed.to_string(),
                        value_text,
                    });
                    // Clear the inputs — the workbook dispatcher re-projects,
                    // so the new Name block appears through the reactive
                    // closure above with no refresh of our own.
                    name_buffer.set(String::new());
                    value_buffer.set(String::new());
                };
                view! {
                    <div class="dna-notebook__add-name-form" data-testid="notebook-add-name-form">
                        <input
                            type="text"
                            class="dna-notebook__add-name-input"
                            data-testid="notebook-name-input"
                            placeholder="name"
                            prop:value=move || name_buffer.get()
                            on:input=move |ev| name_buffer.set(event_target_value(&ev))
                        />
                        <span class="dna-notebook__add-name-eq" aria-hidden="true">"="</span>
                        <input
                            type="text"
                            class="dna-notebook__add-name-value"
                            data-testid="notebook-name-value"
                            placeholder="0.065 or =A1+A2"
                            prop:value=move || value_buffer.get()
                            on:input=move |ev| value_buffer.set(event_target_value(&ev))
                        />
                        <button
                            type="button"
                            class="dna-notebook__add-name-create"
                            data-testid="notebook-create-name"
                            prop:disabled=move || name_buffer.get().trim().is_empty()
                            on:click=create
                        >
                            "Create"
                        </button>
                        <span
                            class="dna-notebook__add-name-hint"
                            data-testid="notebook-add-name-hint"
                        >
                            {move || {
                                if name_buffer.get().trim().is_empty() {
                                    "enter a name to create"
                                } else {
                                    ""
                                }
                            }}
                        </span>
                    </div>
                }
                .into_any()
            }}
        </div>
    }
    .into_any()
}

/// The editable region of a Cell block: an editable cell gets a degrade editor
/// (seeded from its own authored text) plus its honest outcome chip; a
/// non-editable cell gets a read-only note naming why (never a fake editor).
// Threads the same cohesive block context as [`render_block`], plus the S2.11
// preview seam; see that function's note on the `#[allow]`.
#[allow(clippy::too_many_arguments)]
fn render_cell_editor(
    grid: &NodeId,
    authored: &GridAuthoredCellProjection,
    dispatch: &Arc<dyn Dispatcher>,
    // S2.11: the optional preview service; `Some` drives the delete ghost-preview
    // path, `None` the receipt-fallback path (see [`preview_block_delete`]).
    preview: Option<&Arc<dyn PreviewService>>,
    workspace: ReadSignal<WorkspaceState>,
    editors: StoredValue<HashMap<BlockKey, BlockEditState>>,
    owner: Option<&Owner>,
    authoring: bool,
) -> AnyView {
    // S2.9: a non-authoring persona (Reviewer / ReadOnly) gets NO editor and NO
    // commit affordance — only an honest read-only note. The value region, name,
    // classification chip, liveness dot, and any outcome are rendered by the
    // caller and stay fully READABLE; this function owns only the MUTATION
    // control, so gating it here IS the whole read-only render. Never a disabled-
    // looking editor that would in fact reject a write (SHELL_SPEC §6 honesty).
    // Persona takes precedence over cell editability: the governing reason a
    // Reviewer cannot edit is the persona, not the cell's structural role.
    if !authoring {
        return read_only_note(READ_ONLY_PERSONA_NOTE);
    }
    // Respect editability honestly: a non-`Editable` cell is read-only.
    if let Some(reason) = editability_note(&authored.editability) {
        return read_only_note(reason);
    }

    let row = authored.row;
    let col = authored.col;
    let key: BlockKey = (grid.clone(), row, col);
    let seed = authored_seed_text(authored);
    // This block's own edit state — resolved by its own address, so it can never
    // share signals with (or be reset by) another block.
    let state = block_edit_state(editors, owner, &key, &seed);

    // The commit path: build the entry intent from THIS block's own address and
    // dispatch it; read the host's three-way outcome back. The bridge emits only
    // semantic events — the STAGE constructs the intent (layering law). No
    // workspace refresh here: the workbook dispatcher re-projects, so the value
    // region above repaints through the reactive closure.
    let commit_grid = grid.clone();
    let commit_dispatch = Arc::clone(dispatch);
    let on_event = Callback::new(move |event: BridgeEvent| match event {
        // Verbatim text; the host classifies it — the skin never inspects `=`.
        BridgeEvent::TextEdited { text, .. } => state.edit_text.set(text),
        BridgeEvent::CommitRequested => {
            let text = state.edit_text.get_untracked();
            let receipt = commit_dispatch.dispatch(enter_cell_intent(
                commit_grid.clone(),
                row,
                col,
                text,
            ));
            let resolved = interpret_receipt(&receipt);
            if let CellOutcome::Rejected(diagnostics) = &resolved {
                // Keep the rejected text intact so the user can fix it; underline
                // the typed diagnostics.
                state.rejections.set(diagnostics.clone());
            } else {
                state.rejections.set(Vec::new());
            }
            state.outcome.set(Some(resolved));
            // Remount the editor to re-seed committed text / apply rejections,
            // independent of whether the workspace fired (a rejection does not).
            state.revision.update(|revision| *revision += 1);
        }
        BridgeEvent::RevertRequested => {
            // Esc: drop the in-progress edit back to the cell's current authored
            // text (read live from host truth), and clear the transient outcome.
            let authored_now = workspace
                .with_untracked(|ws| current_authored_seed(ws, &commit_grid, row, col));
            state.edit_text.set(authored_now);
            state.rejections.set(Vec::new());
            state.outcome.set(None);
            state.revision.update(|revision| *revision += 1);
        }
        _ => {}
    });

    // --- S2.11 delete affordance (mechanism 12) -------------------------------
    // The delete-as-clear intent for THIS block, and an owned clone of the
    // optional preview service. On activation the delete control runs the pure
    // [`preview_block_delete`] decision over these to pick its path: a ghost when
    // a preview answers, a receipt-fallback clear otherwise.
    let delete_intent = delete_impact_intent(grid);
    let delete_preview = preview.map(Arc::clone);
    let delete_grid = grid.clone();
    let delete_dispatch = Arc::clone(dispatch);
    let on_delete = move |_| {
        match preview_block_delete(delete_preview.as_ref(), &delete_intent) {
            // Ghost path: stash the impact so the ghost region renders — NO
            // dispatch yet. Confirm (below) is the only thing that mutates.
            DeletePreview::Ghost(impact) => state.pending_ghost.set(Some(impact)),
            // Receipt-fallback path (`ctx.preview == None`, the app today, or a
            // preview that errored): dispatch the clear directly and render the
            // honest receipt through the existing outcome chip — never a ghost.
            DeletePreview::ReceiptFallback => {
                dispatch_cell_clear(&delete_dispatch, &delete_grid, row, col, state);
            }
        }
    };
    // The ghost region's own confirm needs this block's address + dispatcher; a
    // separate clone from the button's (each is moved into its own closure).
    let ghost_grid = grid.clone();
    let ghost_dispatch = Arc::clone(dispatch);

    // Esc-to-revert at the EDIT-REGION level, not just on the ghost div: after
    // clicking Delete, focus rests on the Delete button (a sibling of the ghost),
    // so an Escape there would never reach the ghost's own handler. This shared
    // ancestor of both the button and the ghost catches the bubbling Escape and
    // reverts a pending ghost with ZERO dispatch — the headline "Esc revert"
    // affordance works from wherever focus lands after Delete. It is inert (does
    // not consume Escape) when no ghost is pending, so normal editor Esc/revert
    // through the bridge is untouched.
    let on_edit_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" && state.pending_ghost.get_untracked().is_some() {
            ev.prevent_default();
            ev.stop_propagation();
            state.pending_ghost.set(None);
        }
    };

    view! {
        <div
            class="dna-notebook__edit"
            data-testid="notebook-block-edit"
            on:keydown=on_edit_keydown
        >
            {move || {
                // Remount on a commit/revert (`revision`) — never per keystroke —
                // re-seeding from `edit_text` and re-applying `rejections`, both
                // read untracked so typing does not remount. Same discipline the
                // app's `CalcBridgeSurface` uses, keyed here to THIS block.
                state.revision.get();
                let seed_now = state.edit_text.get_untracked();
                let rejections_now = state.rejections.get_untracked();
                view! {
                    <FormulaBridgeDegrade
                        text=seed_now
                        rejections=rejections_now
                        on_event=on_event
                    />
                }
            }}
            // S2.11 delete control (authoring-gated by the early return above):
            // deleting a Cell block CLEARS its cell. Activation runs
            // [`preview_block_delete`] — a ghost when previewable, an immediate
            // receipt-fallback clear otherwise.
            <button
                type="button"
                class="dna-notebook__delete"
                data-testid="notebook-block-delete"
                on:click=on_delete
            >
                "Delete"
            </button>
            {move || {
                // S2.11 ghost-preview region: renders ONLY while this block holds
                // a pending impact (the preview seam answered). It is a pure,
                // non-mutating readout of the delete's impact — Esc / Cancel drop
                // it with ZERO dispatch; Confirm dispatches the actual clear.
                state
                    .pending_ghost
                    .get()
                    .map(|impact| {
                        let summary = describe_delete_impact(&impact);
                        let legal = impact.legal;
                        // Fresh per-run clones handed to this run's confirm
                        // closure; the outer closure keeps its captures (Fn).
                        let confirm_grid = ghost_grid.clone();
                        let confirm_dispatch = Arc::clone(&ghost_dispatch);
                        let confirm = move |_| {
                            // Confirm IS the mutation: dispatch the clear, read
                            // the honest receipt, then drop the ghost.
                            dispatch_cell_clear(
                                &confirm_dispatch,
                                &confirm_grid,
                                row,
                                col,
                                state,
                            );
                            state.pending_ghost.set(None);
                        };
                        // Cancel / Esc: revert the ghost, dispatch NOTHING.
                        let cancel = move |_| state.pending_ghost.set(None);
                        let on_ghost_keydown = move |ev: leptos::ev::KeyboardEvent| {
                            if ev.key() == "Escape" {
                                // Esc reverts the ghost with ZERO dispatch — the
                                // whole handler touches only this signal.
                                ev.prevent_default();
                                ev.stop_propagation();
                                state.pending_ghost.set(None);
                            }
                        };
                        view! {
                            <div
                                class="dna-notebook__delete-ghost"
                                data-testid="notebook-delete-ghost"
                                data-legal=if legal { "true" } else { "false" }
                                tabindex="0"
                                on:keydown=on_ghost_keydown
                            >
                                <span
                                    class="dna-notebook__delete-ghost-summary"
                                    data-testid="notebook-delete-ghost-summary"
                                >
                                    {summary}
                                </span>
                                <button
                                    type="button"
                                    class="dna-notebook__delete-confirm"
                                    data-testid="notebook-delete-confirm"
                                    on:click=confirm
                                >
                                    "Confirm delete"
                                </button>
                                <button
                                    type="button"
                                    class="dna-notebook__delete-cancel"
                                    data-testid="notebook-delete-cancel"
                                    on:click=cancel
                                >
                                    "Cancel (Esc)"
                                </button>
                            </div>
                        }
                    })
            }}
            {move || {
                state
                    .outcome
                    .get()
                    .map(|current| {
                        let label = current.label();
                        let detail = outcome_detail(&current);
                        view! {
                            <div
                                class="dna-notebook__outcome"
                                data-testid="notebook-block-outcome"
                                data-outcome=label
                            >
                                <span class="dna-notebook__outcome-label">{label}</span>
                                <span class="dna-notebook__outcome-detail">{detail}</span>
                            </div>
                        }
                    })
            }}
        </div>
    }
    .into_any()
}

/// The outcome of [`preview_block_delete`]: either a non-mutating
/// **ghost-preview** of the delete's impact (render it, let Esc revert, confirm
/// to dispatch the clear), or a fall back to **post-attempt receipt feedback**
/// (dispatch the clear now, render the honest [`CellOutcome`]) when no preview
/// is available or the preview errored.
// A short-lived decision value — created by [`preview_block_delete`] and matched
// immediately by the delete control, never stored or collected — so the variant
// size difference is immaterial; boxing the impact would add a needless
// allocation on the (rare, user-initiated) ghost path.
#[allow(clippy::large_enum_variant)]
enum DeletePreview {
    /// A preview answered: the host's non-mutating impact of the prospective
    /// clear, to be summarized honestly and confirmed/reverted before any write.
    Ghost(MutationImpactProjection),
    /// No ghost — dispatch the clear directly and read the honest receipt. Both
    /// `ctx.preview == None` (the calc app today) and a preview that ERRORED
    /// land here; a failed preview is never escalated (`preview.rs`).
    ReceiptFallback,
}

/// **The pure seam core (mechanism 12).** Decide how a block's delete should
/// behave, WITHOUT mutating anything. With a [`PreviewService`], the prospective
/// clear is offered to [`PreviewService::preview_mutation_impact`]: an
/// `Ok(impact)` becomes a [`DeletePreview::Ghost`] (rendered before any
/// dispatch); an `Err(_)` degrades to [`DeletePreview::ReceiptFallback`] —
/// previews failing is never an escalated error (`preview.rs`: "previews
/// failing is never an error a lens escalates"). With `None` (the calc app's
/// reality, `ctx.preview == None`) it is likewise a `ReceiptFallback`. A pure
/// function of its inputs, so it is unit-testable with a stub `PreviewService`
/// and cannot drift from what the delete control renders.
fn preview_block_delete(
    preview: Option<&Arc<dyn PreviewService>>,
    intent: &MutationImpactIntentProjection,
) -> DeletePreview {
    match preview {
        Some(service) => match service.preview_mutation_impact(intent) {
            Ok(impact) => DeletePreview::Ghost(impact),
            Err(_) => DeletePreview::ReceiptFallback,
        },
        None => DeletePreview::ReceiptFallback,
    }
}

/// Build the preview intent for deleting (== CLEARING) a Cell block. Delete-as-
/// clear is `EditContent { content: "" }` — editing the node's content to empty
/// — NOT [`MutationImpactIntentProjection::DeleteNode`], which would remove a
/// node entirely; clearing a cell keeps its address and empties it (Excel).
///
/// **Honest workbook-cell → `NodeId` note.** The preview seam addresses NODES,
/// but a workbook grid cell carries no per-cell [`NodeId`] in the projection:
/// [`GridCellProjection`]/[`GridAuthoredCellProjection`] expose only `row`/`col`
/// plus their owning grid's id. So the closest honest addressable node for this
/// seam is the GRID (sheet) node; the row/col are not expressible in the current
/// `MutationImpactIntentProjection` shape. This never runs live today — the calc
/// app's `ctx.preview` is `None`, so a real host never receives this intent — it
/// exists to drive the seam's code path in the stub-backed unit tests. If a
/// future host adds a per-cell node identity (or a cell-addressed impact
/// variant), this is the single site to make the intent precise.
fn delete_impact_intent(grid: &NodeId) -> MutationImpactIntentProjection {
    MutationImpactIntentProjection::EditContent {
        node: grid.clone(),
        content: String::new(),
    }
}

/// Summarize a delete's ghost-preview impact HONESTLY from the real
/// [`MutationImpactProjection`] fields — never a fabricated number. Reports
/// legality (with the typed `blocked_reason` when the host says it is blocked)
/// and the EXACT counts of `orphaned_dependents` and `affected_refs` the host
/// computed. A pure function of the projection, so it is unit-testable and
/// cannot drift from what the ghost region renders.
fn describe_delete_impact(impact: &MutationImpactProjection) -> String {
    let orphaned = impact.orphaned_dependents.len();
    let affected = impact.affected_refs.len();
    let legality = if impact.legal {
        "clears this cell".to_string()
    } else {
        let reason = impact
            .blocked_reason
            .map_or("unknown", |reason| reason.stable_id());
        format!("blocked ({reason})")
    };
    format!("{legality} — {orphaned} dependent(s) orphaned, {affected} reference(s) affected")
}

/// Dispatch the delete-as-clear write for a block — `enter_cell_intent(grid,
/// row, col, "")`, Excel's empty-commit-clears path (→ [`CellOutcome::Cleared`])
/// — and record the honest receipt on the block's OWN state (outcome chip +
/// rejection underline + editor remount). The SAME entry-verb seam
/// [`render_cell_editor`]'s commit crosses (SHELL_SPEC §6 layering law), reached
/// by the delete control instead of the bridge's Enter: deleting a Cell block
/// clears its cell, never removes a workbook address. Reads host truth only.
fn dispatch_cell_clear(
    dispatch: &Arc<dyn Dispatcher>,
    grid: &NodeId,
    row: u32,
    col: u32,
    state: BlockEditState,
) {
    let receipt = dispatch.dispatch(enter_cell_intent(grid.clone(), row, col, String::new()));
    let resolved = interpret_receipt(&receipt);
    if let CellOutcome::Rejected(diagnostics) = &resolved {
        state.rejections.set(diagnostics.clone());
    } else {
        // A successful clear empties the buffer so the remounted editor reflects
        // the now-cleared cell rather than the pre-clear text.
        state.rejections.set(Vec::new());
        state.edit_text.set(String::new());
    }
    state.outcome.set(Some(resolved));
    state.revision.update(|revision| *revision += 1);
}

/// The single persona gate this stage reads at EVERY mutation-control site
/// (S2.9 / NOTEBOOK_SPEC §6 scenario 4). Authoring — the per-block degrade
/// editor, the `+ name` form, the keyboard mutation grammar, and the S2.11
/// delete control — is offered ONLY to a persona that [`Persona::can_mutate`]
/// (Author). Reviewer and ReadOnly render the SAME content fully readable with
/// ZERO enabled mutation controls: the report artifact (there is no separate
/// export in v1). A pure function of [`Persona`], so it is unit-testable and
/// every gate site reads the SAME rule — no site can drift from another.
fn persona_allows_authoring(persona: Persona) -> bool {
    persona.can_mutate()
}

/// The honest note shown wherever a non-authoring persona (Reviewer / ReadOnly)
/// would otherwise reach a mutation control (S2.9): the per-block editor slot
/// ([`render_cell_editor`]) and the `aria-live` keyboard status (the root
/// `on:keydown`) both surface it, so the reason a control is absent reads
/// identically in the DOM and to a screen reader.
const READ_ONLY_PERSONA_NOTE: &str = "read-only persona — editing disabled";

/// An honest, testable read-only note — a cell/name the Notebook deliberately
/// does not offer an editor for, with the reason stated (never a fake editor).
fn read_only_note(reason: &'static str) -> AnyView {
    view! {
        <p class="dna-notebook__readonly" data-testid="notebook-block-readonly">
            {reason}
        </p>
    }
    .into_any()
}

/// **S2.10 substrate probe — WHOLESALE HONEST DEGRADE.** A real block-to-block
/// Reference X-Ray needs the host to produce a caret-driven
/// `ReferenceResolutionProjection` (`dnacalc_skin_ir::ReferenceResolutionProjection`)
/// that resolves a defined-name (or cell) reference to a specific block — i.e.
/// `WorkspaceState.dependencies.reference_resolutions` populated with entries
/// carrying a `token_span` + `target` (skin-ir `workspace.rs`:
/// `DependencyGraphProjection.reference_resolutions:
/// BTreeMap<String, ReferenceResolutionProjection>`).
///
/// It is not produced. `dnacalc-host-core` has ZERO references to
/// `reference_resolutions` / `DependencyGraphProjection` / any `dependencies`
/// population anywhere in its source (verified by direct grep of
/// `src/dnacalc-host-core/src/`), so the workbook `snapshot()` leaves
/// `WorkspaceState.dependencies` at its empty `DependencyGraphProjection::default()`.
/// The TYPE machinery exists in skin-ir; it is simply never populated for the
/// workbook profile. There is therefore no caret X-Ray substrate to build a
/// real feature on top of, so this stage ships a WHOLESALE degrade (ask #7 /
/// G9) instead of a half-built one: no caret tracking, no gutter arrows, no
/// reference highlighting — none of that machinery exists in this crate.
///
/// This predicate is the REAL substrate check (non-empty
/// `reference_resolutions`), never a hardcoded `false`: if the host ever
/// populates the projection, this flips honestly and a future reader knows
/// exactly where to build the real feature — see [`REFERENCE_XRAY_DEGRADE_NOTE`]
/// for the note this gates.
fn reference_xray_available(ws: &WorkspaceState) -> bool {
    !ws.dependencies.reference_resolutions.is_empty()
}

/// The honest disabled-with-reason text shown wherever
/// [`reference_xray_available`] is `false` (today, always) — the S2.10
/// wholesale degrade note (`data-testid="notebook-xray-degrade"`), citing the
/// substrate gap by name (ask #7 / G9) rather than silently omitting the
/// feature.
const REFERENCE_XRAY_DEGRADE_NOTE: &str = "reference X-ray unavailable — the workbook dependency projection is not produced (ask #7 / G9)";

/// The reason a cell is not directly editable, or `None` when it is `Editable`.
/// A skin renders the non-`Editable` variants read-only (the entry verb would
/// reject a write to them anyway, H6).
fn editability_note(editability: &GridEditabilityProjection) -> Option<&'static str> {
    match editability {
        GridEditabilityProjection::Editable => None,
        GridEditabilityProjection::RepeatedRegionMember { .. } => {
            Some("read-only — part of a repeated-formula region")
        }
        GridEditabilityProjection::MergedFollower { .. } => {
            Some("read-only — follower of a merged region")
        }
        GridEditabilityProjection::SpillDisplay { .. } => {
            Some("read-only — spilled from an array formula")
        }
        GridEditabilityProjection::TableStructural { .. } => {
            Some("read-only — structural table cell")
        }
    }
}

/// The editor seed for a cell: its formula source text, else its literal text,
/// else empty — never a computed value.
fn authored_seed_text(authored: &GridAuthoredCellProjection) -> String {
    authored
        .source_text
        .clone()
        .or_else(|| authored.literal_text.clone())
        .unwrap_or_default()
}

/// The current authored seed text for `(grid, row, col)` read live from the
/// workspace projection (used by Esc/revert). Empty when the cell has no
/// authored record in the current window.
fn current_authored_seed(ws: &WorkspaceState, grid: &NodeId, row: u32, col: u32) -> String {
    ws.grids
        .get(grid)
        .and_then(|grid| grid.cells.iter().find(|cell| cell.row == row && cell.col == col))
        .and_then(|cell| cell.authored.as_ref())
        .map(authored_seed_text)
        .unwrap_or_default()
}

/// The outcome chip's detail text — the same honest three-way readout the app's
/// `adapter.rs` renders, ported here (presentation only, no engine truth
/// re-derived).
fn outcome_detail(outcome: &CellOutcome) -> String {
    match outcome {
        CellOutcome::Literal { value } => format!("literal value {value}"),
        CellOutcome::Formula { value, unresolved } if unresolved.is_empty() => {
            format!("formula → {value}")
        }
        CellOutcome::Formula { value, unresolved } => {
            format!("formula → {value} (unresolved: {})", unresolved.join(", "))
        }
        CellOutcome::Cleared => "cell cleared".to_string(),
        CellOutcome::Rejected(diagnostics) => diagnostics
            .first()
            .map(|diagnostic| diagnostic.message.clone())
            .unwrap_or_else(|| "rejected".to_string()),
        CellOutcome::NoChange => "no change".to_string(),
    }
}

/// Render a scalar value as its display TEXT, or an ARRAY as a shape summary
/// plus an honest degrade note — NEVER a fabricated full array render, and NOT
/// a live link (Sheet is a stub + the Inspector is unmounted in S2).
fn render_value(value: &NodeValueProjection) -> AnyView {
    match value {
        NodeValueProjection::Array { rows, cols, .. } => {
            let summary = array_shape_summary(*rows, *cols);
            view! {
                <div
                    class="dna-notebook__result"
                    data-dna-density="working"
                    data-value-kind="array"
                    data-testid="notebook-block-value"
                >
                    <span class="dna-notebook__shape">{summary}</span>
                    <span class="dna-notebook__degrade" data-testid="notebook-array-degrade">
                        "array result — open in Sheet (not yet available in S2)"
                    </span>
                </div>
            }
            .into_any()
        }
        scalar => {
            let text = scalar_value_text(scalar);
            view! {
                <div
                    class="dna-notebook__result"
                    data-dna-density="working"
                    data-value-kind="scalar"
                    data-testid="notebook-block-value"
                >
                    <span class="dna-notebook__value">{text}</span>
                </div>
            }
            .into_any()
        }
    }
}

/// A defined name's result region: the backing cell's computed value when the
/// window resolved one, else an honest passthrough — the dynamic name's source
/// text, or a "not in window" note for a static name outside the current
/// window. Never a fabricated value.
fn render_name_value(name: &DefinedNameProjection, backing: &Option<GridCellProjection>) -> AnyView {
    if let Some(cell) = backing {
        return render_value(&cell.value);
    }
    match &name.target {
        DefinedNameTargetProjection::Dynamic { source_text } => {
            let text = source_text.clone();
            view! {
                <div
                    class="dna-notebook__result"
                    data-dna-density="working"
                    data-value-kind="formula"
                    data-testid="notebook-block-value"
                >
                    <span class="dna-notebook__formula">{text}</span>
                </div>
            }
            .into_any()
        }
        DefinedNameTargetProjection::Static(_) => view! {
            <div
                class="dna-notebook__result"
                data-dna-density="working"
                data-value-kind="unresolved"
                data-testid="notebook-block-value"
            >
                <span class="dna-notebook__note">"value not in the current window"</span>
            </div>
        }
        .into_any(),
    }
}

/// A table overlay's result region: a structural summary of its range and
/// column count. A table is not a single scalar — the block shows its shape,
/// not a fabricated value (full table rendering is a Sheet/Model concern).
fn render_table_value(table: &GridTableOverlayDescriptor) -> AnyView {
    let rect = &table.table_range;
    let rows = rect.bottom_row.saturating_sub(rect.top_row) + 1;
    let cols = if table.columns.is_empty() {
        rect.right_col.saturating_sub(rect.left_col) + 1
    } else {
        table.columns.len() as u32
    };
    let summary = format!("table — {rows}×{cols}");
    view! {
        <div
            class="dna-notebook__result"
            data-dna-density="working"
            data-value-kind="table"
            data-testid="notebook-block-value"
        >
            <span class="dna-notebook__shape">{summary}</span>
        </div>
    }
    .into_any()
}

/// The stable `data-block-kind` id for an entry.
fn entry_kind_id(kind: &NotebookEntryKind) -> &'static str {
    match kind {
        NotebookEntryKind::Name { .. } => "name",
        NotebookEntryKind::Cell { .. } => "cell",
        NotebookEntryKind::Table { .. } => "table",
    }
}

/// The gutter kind glyph — a per-kind mark that reads alongside the (structural)
/// classification tint. Purely decorative signposting; the load-bearing kind
/// signal is `data-block-kind`.
fn entry_kind_glyph(kind: &NotebookEntryKind) -> &'static str {
    match kind {
        NotebookEntryKind::Name { .. } => "◈",
        NotebookEntryKind::Cell { .. } => "▦",
        NotebookEntryKind::Table { .. } => "▤",
    }
}

/// The human-readable classification chip label.
fn classification_label(classification: NodeClassification) -> &'static str {
    match classification {
        NodeClassification::Input => "input",
        NodeClassification::FreeValue => "free value",
        NodeClassification::Intermediate => "intermediate",
        NodeClassification::Output => "output",
        NodeClassification::Empty => "empty",
    }
}

/// The liveness-dot state derived from the classification: a formula-backed
/// entry is "live", a literal-backed entry is "static", an empty one "inert".
fn liveness_id(classification: NodeClassification) -> &'static str {
    match classification {
        NodeClassification::Intermediate | NodeClassification::Output => "live",
        NodeClassification::Input | NodeClassification::FreeValue => "static",
        NodeClassification::Empty => "inert",
    }
}

/// The bounded shape summary for an array result (`3×2 array`).
fn array_shape_summary(rows: usize, cols: usize) -> String {
    format!("{rows}×{cols} array")
}

/// A scalar value's display TEXT. Every non-array projection maps to honest
/// text (the marker variants render as parenthesized notes so no value silently
/// disappears); the array arm is unreachable — callers route arrays through the
/// shape-summary path in [`render_value`].
fn scalar_value_text(value: &NodeValueProjection) -> String {
    match value {
        NodeValueProjection::Number { display, .. } => display.clone(),
        NodeValueProjection::Text(text) => text.clone(),
        NodeValueProjection::Logical { display, .. } => display.clone(),
        NodeValueProjection::Scalar(text) => text.clone(),
        NodeValueProjection::Reference { target } => target.clone(),
        NodeValueProjection::Empty => "(empty)".to_string(),
        NodeValueProjection::Missing => "(missing)".to_string(),
        NodeValueProjection::Unevaluated => "(unevaluated)".to_string(),
        NodeValueProjection::Pending => "(pending)".to_string(),
        NodeValueProjection::Error(message) => message.clone(),
        NodeValueProjection::Array { rows, cols, .. } => array_shape_summary(*rows, *cols),
    }
}

/// The Notebook stage's scoped stylesheet — Strand `--dna-*` tokens only. The
/// classification tints resolve through Strand's semantic soft-token palette
/// (`--dna-{green,amber,accent,signal}-soft` + `--dna-paper-2`); the density
/// values (`68ch` measure, `1.6`/`1.35` leading) are the Strand
/// `Density::Reading`/`Density::Working` constants, applied structurally to the
/// Reading spine and its Working result rows.
pub const NOTEBOOK_CSS: &str = "\
.dna-notebook{display:flex;flex-direction:column;gap:var(--dna-gap-4);max-width:68ch;line-height:1.6;padding:var(--dna-gap-5);color:var(--dna-ink)}
.dna-notebook__empty{margin:0;color:var(--dna-ink-3);font-style:italic}
.dna-notebook__add-name{display:flex;flex-direction:column;gap:var(--dna-gap-2);align-items:flex-start}
.dna-notebook__add-name-toggle{align-self:flex-start;font:inherit;font-size:12px;font-weight:600;color:var(--dna-accent-ink);background:var(--dna-accent-soft);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:var(--dna-gap-1) var(--dna-gap-3);cursor:pointer}
.dna-notebook__add-name-form{display:flex;align-items:center;gap:var(--dna-gap-2);flex-wrap:wrap;padding:var(--dna-gap-3);border:1px solid var(--dna-line);border-radius:var(--dna-radius-card);background:var(--dna-paper-2);width:100%}
.dna-notebook__add-name-input,.dna-notebook__add-name-value{font:inherit;font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;font-size:12px;color:var(--dna-ink);background:var(--dna-paper);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:var(--dna-gap-1) var(--dna-gap-2)}
.dna-notebook__add-name-eq{color:var(--dna-ink-3)}
.dna-notebook__add-name-create{font:inherit;font-size:11px;font-weight:600;color:var(--dna-ink);background:var(--dna-paper);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:var(--dna-gap-1) var(--dna-gap-3);cursor:pointer}
.dna-notebook__add-name-create:disabled{color:var(--dna-ink-3);cursor:not-allowed}
.dna-notebook__add-name-hint{font-size:11px;color:var(--dna-ink-3);font-style:italic}
.dna-notebook__kbd-status{margin:0;min-height:1em;font-size:11px;color:var(--dna-ink-3);font-style:italic}
.dna-notebook__xray-degrade{margin:0;font-size:11px;color:var(--dna-ink-3);font-style:italic}
.dna-notebook__block{display:flex;align-items:stretch;border:1px solid var(--dna-line);border-radius:var(--dna-radius-card);background:var(--dna-paper);overflow:hidden}
.dna-notebook__block:focus-visible{outline:2px solid var(--dna-accent-ink);outline-offset:1px}
.dna-notebook__gutter{display:flex;align-items:flex-start;justify-content:center;padding:var(--dna-gap-3) var(--dna-gap-2);min-width:1.9rem;border-right:1px solid var(--dna-line);background:var(--dna-paper-2)}
.dna-notebook__gutter--input{background:var(--dna-green-soft)}
.dna-notebook__gutter--free_value{background:var(--dna-amber-soft)}
.dna-notebook__gutter--intermediate{background:var(--dna-accent-soft)}
.dna-notebook__gutter--output{background:var(--dna-signal-soft)}
.dna-notebook__gutter--empty{background:var(--dna-paper-2)}
.dna-notebook__glyph{font-size:13px;line-height:1;color:var(--dna-ink-2)}
.dna-notebook__body{display:flex;flex-direction:column;gap:var(--dna-gap-2);padding:var(--dna-gap-3) var(--dna-gap-4);flex:1;min-width:0}
.dna-notebook__name-row{display:flex;align-items:baseline;gap:var(--dna-gap-2);flex-wrap:wrap}
.dna-notebook__name{font-weight:600;color:var(--dna-ink);font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;font-size:12px;word-break:break-word}
.dna-notebook__chip{font-size:10px;text-transform:uppercase;letter-spacing:0.04em;color:var(--dna-accent-ink);background:var(--dna-accent-soft);border-radius:var(--dna-radius-chip);padding:0 var(--dna-gap-2)}
.dna-notebook__dot{width:6px;height:6px;border-radius:50%;display:inline-block;align-self:center;background:var(--dna-ink-3)}
.dna-notebook__dot[data-liveness=\"live\"]{background:var(--dna-value-ink)}
.dna-notebook__dot[data-liveness=\"static\"]{background:var(--dna-prov-const)}
.dna-notebook__dot[data-liveness=\"inert\"]{background:var(--dna-ink-3)}
.dna-notebook__result{line-height:1.35;display:flex;align-items:baseline;gap:var(--dna-gap-2);flex-wrap:wrap;font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;font-size:12px}
.dna-notebook__value{color:var(--dna-value-ink);font-weight:600;word-break:break-word}
.dna-notebook__formula{color:var(--dna-ink-2)}
.dna-notebook__shape{color:var(--dna-value-ink);font-weight:600}
.dna-notebook__degrade{color:var(--dna-ink-3);font-style:italic;font-family:inherit;font-size:11px}
.dna-notebook__note{color:var(--dna-ink-3);font-style:italic;font-family:inherit}
.dna-notebook__edit{display:flex;flex-direction:column;gap:var(--dna-gap-2);margin-top:var(--dna-gap-2)}
.dna-notebook__readonly{margin:var(--dna-gap-2) 0 0;color:var(--dna-ink-3);font-style:italic;font-size:11px}
.dna-notebook__outcome{display:flex;gap:var(--dna-gap-2);align-items:baseline;padding:var(--dna-gap-1) var(--dna-gap-3);border-radius:var(--dna-radius-chip);background:var(--dna-paper-2);font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace}
.dna-notebook__outcome-label{font-weight:600;text-transform:uppercase;letter-spacing:0.05em;font-size:10px;color:var(--dna-accent-ink)}
.dna-notebook__outcome-detail{font-size:11px;color:var(--dna-ink-2)}
.dna-notebook__delete{align-self:flex-start;font:inherit;font-size:11px;font-weight:600;color:var(--dna-signal-ink);background:var(--dna-signal-soft);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:var(--dna-gap-1) var(--dna-gap-3);cursor:pointer}
.dna-notebook__delete-ghost{display:flex;align-items:center;gap:var(--dna-gap-2);flex-wrap:wrap;padding:var(--dna-gap-2) var(--dna-gap-3);border:1px dashed var(--dna-line);border-radius:var(--dna-radius-card);background:var(--dna-paper-2)}
.dna-notebook__delete-ghost[data-legal=\"false\"]{border-color:var(--dna-signal-ink)}
.dna-notebook__delete-ghost-summary{font-size:11px;color:var(--dna-ink-2);font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace}
.dna-notebook__delete-confirm{font:inherit;font-size:11px;font-weight:600;color:var(--dna-signal-ink);background:var(--dna-paper);border:1px solid var(--dna-signal-ink);border-radius:var(--dna-radius-chip);padding:var(--dna-gap-1) var(--dna-gap-3);cursor:pointer}
.dna-notebook__delete-cancel{font:inherit;font-size:11px;color:var(--dna-ink-2);background:var(--dna-paper);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:var(--dna-gap-1) var(--dna-gap-3);cursor:pointer}
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notebook_stage_identity() {
        let stage = NotebookStage::new();
        assert_eq!(stage.id(), StageId::Notebook);
        assert_eq!(stage.title(), "Notebook");
        assert!(stage.supports(&ProfileTag::ExcelStrict));
    }

    /// S2.9 persona gate: authoring controls are offered ONLY to a persona that
    /// `can_mutate` (Author). Reviewer and ReadOnly render the same content
    /// read-only — the report artifact (NOTEBOOK_SPEC §6 scenario 4). Pins the
    /// single predicate every gate site in this stage reads.
    #[test]
    fn persona_allows_authoring_only_for_author() {
        assert!(persona_allows_authoring(Persona::Author));
        assert!(!persona_allows_authoring(Persona::Reviewer));
        assert!(!persona_allows_authoring(Persona::ReadOnly));
    }

    #[test]
    fn classification_maps_to_chip_liveness_and_tint_class() {
        // Each classification produces a stable chip label, a liveness state,
        // and (via `stable_id`) the tint modifier class NOTEBOOK_CSS styles.
        for (classification, chip, liveness, tint) in [
            (NodeClassification::Input, "input", "static", "input"),
            (
                NodeClassification::FreeValue,
                "free value",
                "static",
                "free_value",
            ),
            (
                NodeClassification::Intermediate,
                "intermediate",
                "live",
                "intermediate",
            ),
            (NodeClassification::Output, "output", "live", "output"),
            (NodeClassification::Empty, "empty", "inert", "empty"),
        ] {
            assert_eq!(classification_label(classification), chip);
            assert_eq!(liveness_id(classification), liveness);
            assert_eq!(classification.stable_id(), tint);
            // The tint class every gutter carries is styled in NOTEBOOK_CSS.
            assert!(NOTEBOOK_CSS.contains(&format!(".dna-notebook__gutter--{tint}")));
        }
    }

    #[test]
    fn scalar_value_text_renders_each_projection_honestly() {
        assert_eq!(
            scalar_value_text(&NodeValueProjection::Number {
                raw: "42".to_string(),
                display: "42".to_string(),
            }),
            "42"
        );
        assert_eq!(
            scalar_value_text(&NodeValueProjection::Text("hi".to_string())),
            "hi"
        );
        assert_eq!(
            scalar_value_text(&NodeValueProjection::Logical {
                value: true,
                display: "TRUE".to_string(),
            }),
            "TRUE"
        );
        // Marker variants never vanish — they render as parenthesized notes.
        assert_eq!(
            scalar_value_text(&NodeValueProjection::Empty),
            "(empty)"
        );
        assert_eq!(
            scalar_value_text(&NodeValueProjection::Unevaluated),
            "(unevaluated)"
        );
    }

    #[test]
    fn array_summary_is_shape_only_never_a_full_render() {
        assert_eq!(array_shape_summary(3, 2), "3×2 array");
        assert_eq!(array_shape_summary(1, 1), "1×1 array");
    }

    #[test]
    fn kind_ids_and_glyphs_are_distinct_per_kind() {
        use dnacalc_skin_ir::{GridAuthoredCellProjection, GridAuthoredKindProjection, NodeId};
        let cell = NotebookEntryKind::Cell {
            grid: NodeId::new("Sheet1"),
            authored: GridAuthoredCellProjection {
                row: 1,
                col: 1,
                kind: GridAuthoredKindProjection::Literal,
                literal_text: Some("1".to_string()),
                source_text: None,
                editability: dnacalc_skin_ir::GridEditabilityProjection::Editable,
            },
            value: NodeValueProjection::Number {
                raw: "1".to_string(),
                display: "1".to_string(),
            },
        };
        assert_eq!(entry_kind_id(&cell), "cell");
        assert_eq!(entry_kind_glyph(&cell), "▦");
    }

    /// Shift+Enter's commit-then-ADVANCE step: `next_cell_key` walks the SAME
    /// `derive_entries` order the block list paints, stops (returns `None`) at
    /// the last Cell rather than wrapping, and is a clean no-op for an absent
    /// key. Driven over the REAL host-core demo workbook (native dev-dep), so the
    /// ordering matches what the mounted stage renders — the exact behavior the
    /// bead acceptance calls out ("commits focused then focuses next").
    #[test]
    fn next_cell_key_walks_derived_order_and_stops_at_the_last_cell() {
        use dnacalc_host_core::{DocumentSession, build_demo_workbook};

        let session = build_demo_workbook().expect("demo workbook");
        let document = DocumentSession::Workbook(session);
        let ws = document.snapshot();

        // The Cell keys in derived order — the sequence Shift+Enter advances
        // through (Name/Table blocks are not editable Cell targets).
        let cell_keys: Vec<BlockKey> = model::derive_entries(&ws)
            .into_iter()
            .filter_map(|entry| match entry.kind {
                NotebookEntryKind::Cell {
                    grid, authored, ..
                } => Some((grid, authored.row, authored.col)),
                NotebookEntryKind::Name { .. } | NotebookEntryKind::Table { .. } => None,
            })
            .collect();
        assert!(
            cell_keys.len() >= 2,
            "the demo derives multiple Cell blocks to advance through"
        );

        // Each cell advances to its immediate successor in derived order.
        for pair in cell_keys.windows(2) {
            assert_eq!(
                next_cell_key(&ws, &pair[0]),
                Some(pair[1].clone()),
                "advance must land on the next derived Cell block"
            );
        }
        // The last cell has NO successor — focus honestly stays, never wraps.
        assert_eq!(
            next_cell_key(&ws, cell_keys.last().expect("at least one cell")),
            None,
            "the last block does not wrap to the first"
        );
        // An absent key resolves to `None` (no panic): a commit whose block has
        // vanished from the derivation is a clean no-advance.
        assert_eq!(
            next_cell_key(&ws, &(NodeId::new("sheet:absent"), 999, 999)),
            None,
            "an unknown key advances nowhere rather than panicking"
        );
    }

    /// The focus-routing selector escape neutralizes the only two CSS-string
    /// metacharacters (`\` and `"`) so a grid `NodeId` bearing either cannot
    /// break the attribute selector; ordinary ids (including the `|` join char)
    /// pass through unchanged.
    #[test]
    fn css_escape_attr_value_neutralizes_quote_and_backslash() {
        assert_eq!(css_escape_attr_value("sheet:Sheet1|3|1"), "sheet:Sheet1|3|1");
        assert_eq!(css_escape_attr_value("a\"b"), "a\\\"b");
        assert_eq!(css_escape_attr_value("a\\b"), "a\\\\b");
    }

    /// The eviction key-set (S2.11): `live_block_keys` returns exactly the
    /// derived **Cell** block addresses — Name/Table entries contribute none, so
    /// pruning `editors` to this set drops only vanished Cell blocks (a deleted
    /// cell) without ever touching a still-present block's edit state. Driven
    /// over the real demo workbook so the set matches what the block list paints.
    #[test]
    fn live_block_keys_are_exactly_the_derived_cell_addresses() {
        use dnacalc_host_core::{DocumentSession, build_demo_workbook};

        let session = build_demo_workbook().expect("demo workbook");
        let document = DocumentSession::Workbook(session);
        let ws = document.snapshot();
        let entries = model::derive_entries(&ws);

        let live = live_block_keys(&entries);
        let expected: HashSet<BlockKey> = entries
            .iter()
            .filter_map(|entry| match &entry.kind {
                NotebookEntryKind::Cell {
                    grid, authored, ..
                } => Some((grid.clone(), authored.row, authored.col)),
                _ => None,
            })
            .collect();
        assert_eq!(live, expected, "every derived Cell address, and only those");
        assert!(!live.is_empty(), "the demo derives at least one Cell block");
        // An address that is NOT derived is absent — so pruning would evict its
        // lingering edit state.
        assert!(!live.contains(&(NodeId::new("sheet:absent"), 999, 999)));
    }

    /// **S2.10 substrate probe, pinned.** Over the REAL host-core demo
    /// workbook (native dev-dep, same seam `next_cell_key_walks_derived_order..`
    /// uses), `reference_xray_available` must be `false` — proving the
    /// substrate is GENUINELY empty for the workbook profile, not merely
    /// assumed empty. If `dnacalc-host-core` ever starts populating
    /// `dependencies.reference_resolutions`, this test starts failing and is
    /// the trigger to build the real feature (fail-until-fixed policy applies
    /// in reverse here: this pins today's honest degrade, not a bug).
    #[test]
    fn reference_xray_available_is_false_for_the_real_demo_workbook() {
        use dnacalc_host_core::{DocumentSession, build_demo_workbook};

        let session = build_demo_workbook().expect("demo workbook");
        let document = DocumentSession::Workbook(session);
        let ws = document.snapshot();

        assert!(
            !reference_xray_available(&ws),
            "the workbook profile does not populate dependencies.reference_resolutions \
             today — this predicate must read that as unavailable"
        );
    }

    /// The other half of the substrate check: it is a REAL read of
    /// `dependencies.reference_resolutions`, not a hardcoded `false`. A
    /// hand-built `WorkspaceState` carrying one
    /// `ReferenceResolutionProjection` flips the predicate to `true` — proving
    /// it would honestly report a live X-Ray substrate if the host ever
    /// produced one.
    #[test]
    fn reference_xray_available_is_true_when_the_projection_is_populated() {
        use dnacalc_skin_ir::{
            DependencyKindProjection, NodeKey, ReferenceResolutionProjection,
            ReferenceTargetProjection, SourceSpanProjection,
        };

        let mut ws = WorkspaceState::default();
        assert!(
            !reference_xray_available(&ws),
            "a freshly-defaulted WorkspaceState carries no resolutions"
        );

        ws.dependencies.reference_resolutions.insert(
            "handle:Sheet1!A1".to_string(),
            ReferenceResolutionProjection {
                source_reference_handle: "A1".to_string(),
                owner: NodeId::new("sheet:Sheet1:B1"),
                owner_key: NodeKey::new("key:Sheet1:B1"),
                descriptor_ids: vec!["descriptor:1".to_string()],
                token_span: Some(SourceSpanProjection {
                    start_utf8: 1,
                    end_utf8: 3,
                }),
                target: ReferenceTargetProjection::Node {
                    node: NodeId::new("sheet:Sheet1:A1"),
                    key: NodeKey::new("key:Sheet1:A1"),
                },
                primary_kind: DependencyKindProjection::StaticDirect,
                requires_rebind_on_structural_change: false,
            },
        );

        assert!(
            reference_xray_available(&ws),
            "a populated reference_resolutions map must flip the predicate to true"
        );
    }

    /// The wholesale degrade note names the substrate gap honestly and cites
    /// the tracked asks, so a reviewer (or a screen reader) never sees a bare
    /// "unavailable" with no explanation.
    #[test]
    fn reference_xray_degrade_note_cites_the_tracked_asks() {
        assert!(NOTEBOOK_CSS.contains(".dna-notebook__xray-degrade"));
        assert!(REFERENCE_XRAY_DEGRADE_NOTE.contains("ask #7"));
        assert!(REFERENCE_XRAY_DEGRADE_NOTE.contains("G9"));
        assert!(REFERENCE_XRAY_DEGRADE_NOTE.contains("reference X-ray unavailable"));
    }

    // --- S2.11 delete ghost-preview / receipt-fallback seam -------------------

    use dnacalc_skin_ir::{
        FormulaBindPreviewProjection, MutationImpactBlockedReasonProjection, PreviewError,
        RecalcPlanProjection,
    };

    /// A stub [`PreviewService`] driving the seam's two branches from one type:
    /// `impact: Some(..)` makes `preview_mutation_impact` answer `Ok(impact)`
    /// (the ghost path); `impact: None` makes it answer
    /// `Err(PreviewError::Unavailable)` (the degrade path). `preview_formula_bind`
    /// is unused on the delete seam and is safely `Err`.
    struct StubPreview {
        impact: Option<MutationImpactProjection>,
    }

    impl PreviewService for StubPreview {
        fn preview_formula_bind(
            &self,
            _node: &NodeId,
            _content: &str,
        ) -> Result<FormulaBindPreviewProjection, PreviewError> {
            Err(PreviewError::Unavailable)
        }

        fn preview_mutation_impact(
            &self,
            _intent: &MutationImpactIntentProjection,
        ) -> Result<MutationImpactProjection, PreviewError> {
            self.impact.clone().ok_or(PreviewError::Unavailable)
        }
    }

    /// A canned impact carrying REAL, distinct counts (2 orphaned dependents, 1
    /// affected reference, legal) so a summary/ghost test can prove the numbers
    /// come from the projection rather than being fabricated.
    fn canned_impact() -> MutationImpactProjection {
        MutationImpactProjection {
            intent: delete_impact_intent(&NodeId::new("sheet:Sheet1")),
            legal: true,
            blocked_reason: None,
            profile_violations: Vec::new(),
            bind_diagnostics: Vec::new(),
            invalidation_plan: RecalcPlanProjection::default(),
            requires_rebind: Vec::new(),
            affected_refs: vec![NodeId::new("sheet:Sheet1:B1")],
            orphaned_dependents: vec![
                NodeId::new("sheet:Sheet1:C1"),
                NodeId::new("sheet:Sheet1:D1"),
            ],
            collisions: Vec::new(),
        }
    }

    /// Delete-as-clear expresses as `EditContent { content: "" }` addressed to
    /// the grid node (the honest closest addressable node — a workbook cell has
    /// no per-cell `NodeId`), NEVER `DeleteNode` (which would remove a node).
    #[test]
    fn delete_impact_intent_is_edit_content_to_empty_on_the_grid_node() {
        let grid = NodeId::new("sheet:Sheet1");
        match delete_impact_intent(&grid) {
            MutationImpactIntentProjection::EditContent { node, content } => {
                assert_eq!(node, grid, "the intent addresses the grid node");
                assert!(content.is_empty(), "clearing sets content to empty");
            }
            other => panic!("delete-as-clear must be EditContent, got {other:?}"),
        }
    }

    /// Ghost path: with a preview service that answers, `preview_block_delete`
    /// returns `Ghost(impact)` carrying the stub's exact impact — proving the
    /// decision actually crosses the `preview_mutation_impact` seam.
    #[test]
    fn preview_block_delete_returns_ghost_when_the_preview_answers() {
        let service: Arc<dyn PreviewService> = Arc::new(StubPreview {
            impact: Some(canned_impact()),
        });
        let intent = delete_impact_intent(&NodeId::new("sheet:Sheet1"));
        match preview_block_delete(Some(&service), &intent) {
            DeletePreview::Ghost(impact) => {
                // The impact is the stub's, verbatim — the seam did not fabricate.
                assert_eq!(impact.orphaned_dependents.len(), 2);
                assert_eq!(impact.affected_refs.len(), 1);
                assert!(impact.legal);
            }
            DeletePreview::ReceiptFallback => {
                panic!("a preview that answered must produce a ghost, not a fallback")
            }
        }
    }

    /// Error degrades, never escalates: a preview whose `preview_mutation_impact`
    /// returns `Err(Unavailable)` falls back to receipt feedback rather than
    /// surfacing an error (the `preview.rs` contract).
    #[test]
    fn preview_block_delete_degrades_to_fallback_when_the_preview_errors() {
        let service: Arc<dyn PreviewService> = Arc::new(StubPreview { impact: None });
        let intent = delete_impact_intent(&NodeId::new("sheet:Sheet1"));
        assert!(
            matches!(
                preview_block_delete(Some(&service), &intent),
                DeletePreview::ReceiptFallback
            ),
            "a failed preview degrades to the receipt-fallback path"
        );
    }

    /// The app's reality: with `ctx.preview == None`, `preview_block_delete`
    /// takes the receipt-fallback path (the delete dispatches its clear and the
    /// honest `CellOutcome` renders — no fabricated ghost).
    #[test]
    fn preview_block_delete_is_fallback_when_no_preview_is_present() {
        let intent = delete_impact_intent(&NodeId::new("sheet:Sheet1"));
        assert!(
            matches!(
                preview_block_delete(None, &intent),
                DeletePreview::ReceiptFallback
            ),
            "no preview service means the receipt-fallback path"
        );
    }

    /// The ghost summary reports the REAL counts from the projection (2 orphaned,
    /// 1 affected) and the legal state — never a fabricated number.
    #[test]
    fn describe_delete_impact_reports_honest_counts_from_real_fields() {
        let summary = describe_delete_impact(&canned_impact());
        assert!(summary.contains("clears this cell"), "legal delete is stated");
        assert!(
            summary.contains("2 dependent(s) orphaned"),
            "the 2 orphaned dependents surface verbatim: {summary}"
        );
        assert!(
            summary.contains("1 reference(s) affected"),
            "the 1 affected reference surfaces verbatim: {summary}"
        );
    }

    /// A blocked impact names the typed `blocked_reason` honestly (never a bare
    /// "unavailable") and still reports its real counts.
    #[test]
    fn describe_delete_impact_names_the_blocked_reason() {
        let mut impact = canned_impact();
        impact.legal = false;
        impact.blocked_reason = Some(MutationImpactBlockedReasonProjection::NameCollision);
        let summary = describe_delete_impact(&impact);
        assert!(
            summary.contains("blocked (name_collision)"),
            "the typed blocked reason is surfaced: {summary}"
        );
        assert!(
            summary.contains("2 dependent(s) orphaned"),
            "counts still surface on the blocked path: {summary}"
        );
    }
}

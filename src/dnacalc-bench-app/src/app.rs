//! The Bench product composition — `ShellComposition::bench()` (no Registry,
//! no stage switcher, hero two-row bridge) with the formula workbench in FULL
//! mode over the real `dnacalc-bench-host` OneFormula projection.
//!
//! `!Send` handling mirrors the estate's `WorkbookHostDispatcher`: the OxFml
//! session inside [`BenchHost`] is `!Send`, so it lives in a `thread_local`
//! keyed by id and the `Send + Sync` Leptos signals/callbacks carry only that
//! id. On the single wasm main thread the id always resolves on its owner.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use leptos::prelude::*;

use dnacalc_bridge::{BridgeEvent, FormulaBridge};
use dnacalc_shell::{
    BridgeSurface, ProfileTag, RuntimeContext, Shell, ShellComposition, StageContext, StageHandle,
    StageId, StageRegistry, StageSurface,
};
use dnacalc_skin_ir::IntentReceipt;
use dnacalc_skin_ir::SkinVerb;
use dnacalc_skin_ir::formula::{FormulaResultSurface, OneFormulaIntent, OneFormulaProjection};
use dnacalc_skin_ir::intent::{Dispatcher, WorkspaceDelta, WorkspaceIntent};
use dnacalc_skin_ir::protocol::{PersistenceProjection, SkinShellIntent};
use dnacalc_skin_ir::selection::SelectionState;
use dnacalc_skin_ir::state::{SharedSkinState, SharedStateChange, SharedStateOrigin};
use dnacalc_skin_ir::workspace::WorkspaceState;
use dnacalc_skin_leptos::state_handles::SharedSkinStateHandle;
use dnacalc_strand::{Density, Theme};

use crate::adapter::BenchHost;
use crate::xray::{RingStep, XRAY_CSS, XRayPanel, result_array_window_intent, ring_step};

thread_local! {
    /// The `!Send` Bench hosts, owned per-thread and addressed by id so every
    /// `Send + Sync` signal/callback holds only the id.
    static BENCH_HOSTS: RefCell<BTreeMap<u64, BenchHost>> =
        const { RefCell::new(BTreeMap::new()) };
}

static NEXT_BENCH_HOST_ID: AtomicU64 = AtomicU64::new(1);

fn with_bench_host<R>(id: u64, f: impl FnOnce(&mut BenchHost) -> R) -> Option<R> {
    BENCH_HOSTS.with(|hosts| hosts.borrow_mut().get_mut(&id).map(f))
}

/// The typed slot the shell mounts the FULL-mode workbench into. Holds only
/// `Send + Sync` handles; it re-renders (remounts) [`FormulaBridge`] whenever
/// a new projection arrives — the estate's remount-per-projection pattern
/// (the bridge seeds its internal signals from props at mount only).
#[derive(Clone)]
struct BenchBridgeSurface {
    projection: RwSignal<OneFormulaProjection>,
    on_event: Callback<BridgeEvent>,
}

impl BridgeSurface for BenchBridgeSurface {
    fn mount(&self, _ctx: StageContext) -> AnyView {
        let projection = self.projection;
        let on_event = self.on_event;
        view! {
            {move || {
                // Reading `projection` here is the remount trigger: each new
                // projection re-runs this closure, constructing a fresh
                // FormulaBridge that re-seeds from the new surfaces.
                let current = projection.get();
                view! {
                    <FormulaBridge
                        editor=current.editor
                        assist=current.assist
                        drill=current.drill
                        on_event=on_event
                    />
                }
            }}
        }
        .into_any()
    }
}

/// The Bench Result stage (SHELL_SPEC §1: "Result stage fixed"). Renders the
/// current OneFormula result surface — scalar / error / array (windowed) —
/// from host truth, and docks the X-Ray panel (BENCH_SPEC §2/§4) at its bottom
/// when `drill.expanded`. No stage switcher exists in Bench, so this is the
/// only stage.
struct BenchResultStage {
    projection: RwSignal<OneFormulaProjection>,
    /// The current X-Ray ring node id (mirrored as a highlighted drill row).
    ring: RwSignal<Option<String>>,
    /// Bridge-event sink (the X-Ray close button emits `DrillToggled`).
    on_event: Callback<BridgeEvent>,
    /// Sanctioned `OneFormulaIntent` request-verb sink (array window paging).
    on_intent: Callback<OneFormulaIntent>,
}

impl StageSurface for BenchResultStage {
    fn id(&self) -> StageId {
        StageId::BenchResult
    }

    fn title(&self) -> &'static str {
        "Result"
    }

    fn supports(&self, profile: &ProfileTag) -> bool {
        matches!(profile, ProfileTag::OneCalc)
    }

    fn mount(&self, _ctx: StageContext) -> StageHandle {
        let projection = self.projection;
        let ring = self.ring;
        let on_event = self.on_event;
        let on_intent = self.on_intent;
        let view = view! {
            <div class="bench-stage">
                <div class="bench-result" data-testid="bench-result">
                    {move || {
                        let current = projection.get();
                        let space_id = current.formula_space_id.clone();
                        match current.result {
                            FormulaResultSurface::Empty => view! {
                                <p class="bench-result__empty" data-result="empty">
                                    "Enter a formula above to see its result."
                                </p>
                            }
                            .into_any(),
                            FormulaResultSurface::Pending => view! {
                                <p class="bench-result__pending" data-result="pending">
                                    "Evaluating…"
                                </p>
                            }
                            .into_any(),
                            FormulaResultSurface::Display { text, value, .. } => {
                                let kind = core_value_label(&value.core);
                                let value_attr = text.clone();
                                view! {
                                    <div class="bench-result__display" data-result="display">
                                        <span class="bench-result__value" data-value=value_attr>
                                            {text}
                                        </span>
                                        <span class="bench-result__kind">{kind}</span>
                                    </div>
                                }
                                .into_any()
                            }
                            FormulaResultSurface::Error { code, .. } => view! {
                                <div class="bench-result__error" data-result="error">
                                    <span class="bench-result__error-code">{code}</span>
                                </div>
                            }
                            .into_any(),
                            FormulaResultSurface::Array {
                                total_rows,
                                total_cols,
                                label,
                                window,
                                truncated,
                            } => {
                                let shape = format!("{total_rows}\u{00D7}{total_cols}");
                                let pager = result_array_window_intent(
                                    space_id,
                                    &window,
                                    total_rows,
                                    total_cols,
                                );
                                let cells = window.cells.clone();
                                view! {
                                    <div class="bench-result__array" data-result="array">
                                        <div class="bench-result__array-head">
                                            <span>{label}</span>
                                            <span class="bench-result__shape">{shape}</span>
                                            {truncated
                                                .then(|| {
                                                    view! {
                                                        <span
                                                            class="bench-result__trunc"
                                                            data-truncated="true"
                                                        >
                                                            "windowed"
                                                        </span>
                                                    }
                                                })}
                                            {pager
                                                .map(|intent| {
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class="bench-result__more"
                                                            on:click=move |_| on_intent.run(intent.clone())
                                                        >
                                                            "More"
                                                        </button>
                                                    }
                                                })}
                                        </div>
                                        <div class="bench-result__grid" role="grid">
                                            {cells
                                                .into_iter()
                                                .map(|row| {
                                                    view! {
                                                        <div class="bench-result__grid-row" role="row">
                                                            {row
                                                                .into_iter()
                                                                .map(|cell| {
                                                                    view! {
                                                                        <span
                                                                            class="bench-result__grid-cell"
                                                                            role="gridcell"
                                                                        >
                                                                            {cell.display_text}
                                                                        </span>
                                                                    }
                                                                })
                                                                .collect_view()}
                                                        </div>
                                                    }
                                                })
                                                .collect_view()}
                                        </div>
                                    </div>
                                }
                                .into_any()
                            }
                        }
                    }}
                </div>
                {move || {
                    let current = projection.get();
                    current
                        .drill
                        .expanded
                        .then(|| {
                            view! {
                                <XRayPanel
                                    drill=current.drill.clone()
                                    formula_space_id=current.formula_space_id.clone()
                                    ring=ring
                                    on_event=on_event
                                    on_intent=on_intent
                                />
                            }
                        })
                }}
            </div>
        }
        .into_any();
        StageHandle::new(view)
    }
}

/// The `PersistenceProjection` the Bench `Shell` mount actually advertises
/// (bead dtc-lfz.3): `can_save` and `dirty` are the host's real truth
/// verbatim (see [`BenchHost::persistence`]) — Save needs no caller-supplied
/// path, so a capable host genuinely writes `workspace.json` end-to-end.
/// `can_open` is force-honest-false regardless of what the host's raw
/// capability says: `SkinShellIntent::Open` still requires a path, and the
/// Bench product has no file-picker/dialog seam anywhere yet to produce one
/// (open with no path is a real, typed rejection every time — see
/// `dispatch_shell_intent_open_without_a_path_is_a_typed_rejection_not_a_silent_noop`
/// in `adapter.rs`), so advertising the raw value would enable a deck row
/// that can never actually open anything. Follow-up: a Tauri dialog plugin /
/// browser `<input type=file>` seam that resolves a path before dispatch.
fn shell_persistence_from_host(host: &BenchHost) -> PersistenceProjection {
    let mut persistence = host.persistence();
    persistence.can_open = false;
    persistence
}

fn core_value_label(core: &dnacalc_skin_ir::formula::CoreValueProjection) -> &'static str {
    use dnacalc_skin_ir::formula::CoreValueProjection as C;
    match core {
        C::Number { .. } => "number",
        C::Text { .. } => "text",
        C::Logical { .. } => "logical",
        C::Error { .. } => "error",
        C::Empty => "empty",
        C::Missing => "missing",
        C::Reference { .. } => "reference",
        C::Array { .. } => "array",
        C::RichValue { .. } => "rich",
        C::Callable { .. } => "callable",
        C::Other { .. } => "other",
    }
}

/// The Bench app dispatcher (SHELL_SPEC §4: exactly one `Dispatcher`). Bench
/// authors through the bridge's OneFormula seam, so the shell-level intents
/// this handles are narrow: persona switching (written back to shared state so
/// the deck's persona marker is never stale — the host owns that write) and
/// F9/deck Recalculate (flush + re-project the OxFml result). Everything else
/// is a faithful accept-no-op — this product has no workspace model to mutate.
struct BenchDispatcher {
    shared: SharedSkinStateHandle,
    host_id: u64,
    projection: RwSignal<OneFormulaProjection>,
}

impl Dispatcher for BenchDispatcher {
    fn dispatch(&self, intent: WorkspaceIntent) -> IntentReceipt {
        match intent {
            WorkspaceIntent::SetPersona { persona } => {
                // Write-back on accept: the shell mirrors the deck's persona
                // marker from `SharedSkinState.persona`, which only changes
                // when the host applies it here.
                self.shared.apply(
                    SharedStateChange::SetPersona(persona),
                    SharedStateOrigin::Host,
                );
                IntentReceipt::accepted()
            }
            WorkspaceIntent::Recalculate => {
                if let Some(next) = with_bench_host(self.host_id, |host| {
                    host.apply(BridgeEvent::CommitRequested);
                    host.projection()
                }) {
                    self.projection.set(next);
                }
                IntentReceipt::accepted()
            }
            _ => IntentReceipt::accepted(),
        }
    }
}

/// The Bench app root. Composes `ShellComposition::bench()` over the real
/// OneFormula host with the workbench in FULL mode.
#[component]
pub fn BenchApp(
    /// Browser (WASM) or desktop (Tauri) — feeds the keyboard grammar's
    /// hard-reserved / desktop-sanctioned chord law (SHELL_SPEC §5.1).
    runtime: RuntimeContext,
) -> impl IntoView {
    // Own one `!Send` Bench host in the thread-local registry.
    let host_id = NEXT_BENCH_HOST_ID.fetch_add(1, Ordering::Relaxed);
    BENCH_HOSTS.with(|hosts| {
        hosts.borrow_mut().insert(host_id, BenchHost::new());
    });
    let initial = with_bench_host(host_id, |host| host.projection()).unwrap_or_default();
    let projection = RwSignal::new(initial);
    // The host's real Save/Open/dirty capability (SHELL_SPEC §4, bead
    // dtc-lfz.3) — threaded into the Shell's `host_persistence` prop so the
    // command deck's Save/Open track REAL host truth instead of the old
    // hardcoded `false`, and the mast dirty-dot picks up edits even though
    // Bench has no `workbook_calc` projection of its own.
    let initial_persistence = with_bench_host(host_id, |host| shell_persistence_from_host(host))
        .unwrap_or_default();
    let persistence = RwSignal::new(initial_persistence);

    // The current X-Ray ring node id (the enclosing-expression the ']'/'['
    // step-out/in verbs walk; the panel mirrors it as a highlighted row).
    let ring = RwSignal::new(None::<String>);

    // The bridge-event sink: translate through the host adapter, then re-project
    // so the bridge remounts over fresh host truth.
    let on_event = Callback::new(move |event: BridgeEvent| {
        let changed = with_bench_host(host_id, |host| host.apply(event)).unwrap_or(false);
        if changed {
            if let Some(next) = with_bench_host(host_id, |host| host.projection()) {
                projection.set(next);
            }
            if let Some(next) =
                with_bench_host(host_id, |host| shell_persistence_from_host(host))
            {
                persistence.set(next);
            }
        }
    });

    // The command deck's Save/Open dispatch through here (bead dtc-lfz.3):
    // a real `SkinShellIntent` over `BenchHost::dispatch_shell_intent`, which
    // routes through the same `OneCalcSessionHost` seam every other OneCalc
    // host surface uses. Re-projects `persistence` afterward so any
    // capability/`current_path` change becomes visible immediately (a
    // pre-existing, out-of-scope gap: `dirty` itself does not clear on Save
    // in the Bench flow today — see the NOTE on
    // `persistence_tracks_real_dirty_state_on_edit` in `adapter.rs`).
    let on_shell_intent = Callback::new(move |intent: SkinShellIntent| {
        let _ = with_bench_host(host_id, |host| host.dispatch_shell_intent(intent));
        if let Some(next) = with_bench_host(host_id, |host| shell_persistence_from_host(host)) {
            persistence.set(next);
        }
    });

    // The sanctioned `OneFormulaIntent` request-verb sink the X-Ray panel /
    // Result stage drive for array-window paging. The window transport is a
    // documented host degrade (see `adapter::BenchHost::apply_intent`), so an
    // accepted change re-projects while an unattached window request honestly
    // no-ops; either way the panel renders the window the projection carries.
    let on_intent = Callback::new(move |intent: OneFormulaIntent| {
        let changed = with_bench_host(host_id, |host| host.apply_intent(intent)).unwrap_or(false);
        if changed && let Some(next) = with_bench_host(host_id, |host| host.projection()) {
            projection.set(next);
        }
    });

    // Shell verbs the Bench product owns (SHELL_SPEC §5 forwards these through
    // `on_shell_verb`): F8 toggles the X-Ray, ']'/'[' walk the evaluation ring.
    let on_shell_verb = Callback::new(move |verb: SkinVerb| match verb {
        // F8 → ToggleFormulaDrill (BENCH_SPEC §4/§9). Function-key exempt, so
        // it fires even from inside the Bridge edit buffer.
        SkinVerb::FormulaXRay => {
            let changed = with_bench_host(host_id, |host| host.apply(BridgeEvent::DrillToggled))
                .unwrap_or(false);
            if changed && let Some(next) = with_bench_host(host_id, |host| host.projection()) {
                projection.set(next);
            }
            let current = projection.get_untracked();
            if current.drill.expanded {
                // Seed the ring at the caret's node so the panel highlights the
                // current subexpression the moment it opens.
                if ring.get_untracked().is_none() {
                    ring.set(ring_step(
                        &current.drill.tree,
                        None,
                        current.editor.caret_offset,
                        RingStep::Out,
                    ));
                }
            } else {
                ring.set(None);
            }
        }
        // ']' widens to the enclosing expression; '[' narrows to the first
        // child. These reach the shell (and thus here) only when focus is
        // outside a text entry — inside the editor they are formula characters,
        // correctly suppressed by the §5 text-entry guard (see the panel's
        // `tabindex` and this bead's routing note).
        SkinVerb::TraceForward | SkinVerb::TraceBack => {
            let current = projection.get_untracked();
            let step = if matches!(verb, SkinVerb::TraceForward) {
                RingStep::Out
            } else {
                RingStep::In
            };
            let next = ring_step(
                &current.drill.tree,
                ring.get_untracked().as_deref(),
                current.editor.caret_offset,
                step,
            );
            ring.set(next);
        }
        _ => {}
    });

    // Empty read-side: Bench carries no workspace model (it authors one
    // formula through the bridge), so these are honest empties the mast/strip
    // read from — document name resolves to "Untitled", most strip slots read
    // absent rather than a fabricated OK.
    let workspace = RwSignal::new(WorkspaceState::default());
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(0));
    let selection = RwSignal::new(SelectionState::default());
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());

    let dispatch: Arc<dyn Dispatcher> = Arc::new(BenchDispatcher {
        shared,
        host_id,
        projection,
    });

    let mut composition = ShellComposition::bench();
    composition.bridge_slot.surface = Some(Arc::new(BenchBridgeSurface {
        projection,
        on_event,
    }));

    let stages = StageRegistry::new().with_stage(Arc::new(BenchResultStage {
        projection,
        ring,
        on_event,
        on_intent,
    }));

    view! {
        <style>{BENCH_APP_CSS}</style>
        <style>{XRAY_CSS}</style>
        <Shell
            composition=composition
            stages=stages
            workspace=workspace.read_only()
            latest_delta=latest_delta.read_only()
            selection=selection.read_only()
            shared=shared
            dispatch=dispatch
            theme=Theme::CockpitLight
            density=Density::Working
            runtime=runtime
            host_persistence=persistence.read_only()
            on_shell_intent=Some(on_shell_intent)
            on_shell_verb=Some(on_shell_verb)
        />
    }
}

/// The Bench Result stage's own component-scoped styles (resolve through the
/// Strand `--dna-*` cascade the shell root emits).
const BENCH_APP_CSS: &str = "\
.bench-stage{display:flex;flex-direction:column;height:100%;min-height:0}
.bench-result{padding:var(--dna-gap-5);display:flex;flex-direction:column;gap:var(--dna-gap-3);flex:1;min-height:0;overflow:auto}
.bench-result__empty,.bench-result__pending{color:var(--dna-ink-3);font-style:italic}
.bench-result__display{display:flex;align-items:baseline;gap:var(--dna-gap-4)}
.bench-result__value{font-size:28px;font-weight:600;color:var(--dna-value-ink)}
.bench-result__kind{font-size:12px;color:var(--dna-ink-3);text-transform:uppercase;letter-spacing:0.05em}
.bench-result__error-code{font-size:20px;font-weight:600;color:var(--dna-red-ink)}
.bench-result__array{display:flex;flex-direction:column;gap:var(--dna-gap-3)}
.bench-result__array-head{display:flex;gap:var(--dna-gap-3);align-items:baseline}
.bench-result__shape{color:var(--dna-ink-3);font-size:12px}
.bench-result__trunc{color:var(--dna-prov-ext);font-size:11px;text-transform:uppercase}
.bench-result__more{font:inherit;font-size:11px;color:var(--dna-accent-ink);background:var(--dna-paper-2);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:0 var(--dna-gap-2);cursor:pointer}
.bench-result__grid{display:flex;flex-direction:column;gap:1px;overflow-x:auto;align-self:flex-start}
.bench-result__grid-row{display:flex;gap:1px}
.bench-result__grid-cell{min-width:3em;padding:0 var(--dna-gap-3);background:var(--dna-paper-2);color:var(--dna-ink);font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;text-align:right}
";

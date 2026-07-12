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
use dnacalc_skin_ir::formula::{FormulaResultSurface, OneFormulaProjection};
use dnacalc_skin_ir::intent::{Dispatcher, WorkspaceDelta, WorkspaceIntent};
use dnacalc_skin_ir::selection::SelectionState;
use dnacalc_skin_ir::state::{SharedSkinState, SharedStateChange, SharedStateOrigin};
use dnacalc_skin_ir::workspace::WorkspaceState;
use dnacalc_skin_leptos::state_handles::SharedSkinStateHandle;
use dnacalc_strand::{Density, Theme};

use crate::adapter::BenchHost;

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
/// current OneFormula result surface — scalar / error / array — from host
/// truth. No stage switcher exists in Bench, so this is the only stage.
struct BenchResultStage {
    projection: RwSignal<OneFormulaProjection>,
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
        let view = view! {
            <div class="bench-result" data-testid="bench-result">
                {move || {
                    let result = projection.get().result;
                    match result {
                        FormulaResultSurface::Empty => view! {
                            <p class="bench-result__empty" data-result="empty">
                                "Enter a formula above to see its result."
                            </p>
                        }
                        .into_any(),
                        FormulaResultSurface::Pending => view! {
                            <p class="bench-result__pending" data-result="pending">"Evaluating…"</p>
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
                            ..
                        } => view! {
                            <div class="bench-result__array" data-result="array">
                                <span>{label}</span>
                                <span class="bench-result__shape">
                                    {format!("{total_rows}×{total_cols}")}
                                </span>
                            </div>
                        }
                        .into_any(),
                    }
                }}
            </div>
        }
        .into_any();
        StageHandle::new(view)
    }
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
                if let Some(next) =
                    with_bench_host(self.host_id, |host| {
                        host.apply(BridgeEvent::CommitRequested);
                        host.projection()
                    })
                {
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

    // The bridge-event sink: translate through the host adapter, then re-project
    // so the bridge remounts over fresh host truth.
    let on_event = Callback::new(move |event: BridgeEvent| {
        let changed = with_bench_host(host_id, |host| host.apply(event)).unwrap_or(false);
        if changed
            && let Some(next) = with_bench_host(host_id, |host| host.projection())
        {
            projection.set(next);
        }
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

    let stages = StageRegistry::new().with_stage(Arc::new(BenchResultStage { projection }));

    view! {
        <style>{BENCH_APP_CSS}</style>
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
        />
    }
}

/// The Bench Result stage's own component-scoped styles (resolve through the
/// Strand `--dna-*` cascade the shell root emits).
const BENCH_APP_CSS: &str = "\
.bench-result{padding:var(--dna-gap-5);display:flex;flex-direction:column;gap:var(--dna-gap-3)}
.bench-result__empty,.bench-result__pending{color:var(--dna-ink-3);font-style:italic}
.bench-result__display{display:flex;align-items:baseline;gap:var(--dna-gap-4)}
.bench-result__value{font-size:28px;font-weight:600;color:var(--dna-value-ink)}
.bench-result__kind{font-size:12px;color:var(--dna-ink-3);text-transform:uppercase;letter-spacing:0.05em}
.bench-result__error-code{font-size:20px;font-weight:600;color:var(--dna-red-ink)}
.bench-result__array{display:flex;gap:var(--dna-gap-3);align-items:baseline}
.bench-result__shape{color:var(--dna-ink-3);font-size:12px}
";

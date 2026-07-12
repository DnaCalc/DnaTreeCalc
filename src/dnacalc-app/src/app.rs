//! The DNA Calc product composition — `ShellComposition::calc()` with two stub
//! stages over the real `dnacalc-host-core` demo workbook (driven through
//! `dnatreecalc-host`'s `WorkbookHostDispatcher`), and the formula workbench in
//! DEGRADE mode editing one workbook cell via `EnterGridCell` with the
//! three-way outcome rendered honestly.

use std::sync::Arc;

use leptos::prelude::*;

use dnacalc_bridge::{BridgeEvent, FormulaBridgeDegrade};
use dnacalc_shell::{
    BridgeSurface, ProfileTag, RuntimeContext, Shell, ShellComposition, StageContext, StageHandle,
    StageId, StageRegistry, StageSurface,
};
use dnacalc_skin_ir::IntentReceipt;
use dnacalc_skin_ir::intent::{Dispatcher, WorkspaceDelta, WorkspaceIntent};
use dnacalc_skin_ir::selection::SelectionState;
use dnacalc_skin_ir::state::{SharedSkinState, SharedStateChange, SharedStateOrigin};
use dnacalc_skin_ir::workspace::{GridEntryDiagnosticProjection, WorkspaceState};
use dnacalc_skin_leptos::state_handles::SharedSkinStateHandle;
use dnacalc_strand::{Density, Theme};
use dnatreecalc_host::app::WorkbookHostDispatcher;

use crate::adapter::{CellOutcome, enter_grid_cell_intent, interpret_receipt};

/// Wraps the workbook dispatcher to make persona switching real: `SetPersona`
/// is written back into `SharedSkinState.persona` on accept (SHELL_SPEC §4 —
/// the deck's persona marker mirrors that field and only changes when the host
/// applies it). Every other intent delegates to the workbook dispatcher.
struct PersonaDispatcher {
    inner: Arc<dyn Dispatcher>,
    shared: SharedSkinStateHandle,
}

impl Dispatcher for PersonaDispatcher {
    fn dispatch(&self, intent: WorkspaceIntent) -> IntentReceipt {
        if let WorkspaceIntent::SetPersona { persona } = &intent {
            self.shared.apply(
                SharedStateChange::SetPersona(*persona),
                SharedStateOrigin::Host,
            );
            return IntentReceipt::accepted();
        }
        self.inner.dispatch(intent)
    }
}

/// A stub Calc stage (SHELL_SPEC §3): renders its identity plus a live readout
/// of shared continuity state (collapse set / selection set sizes) so the
/// stage-switch continuity guarantee is observable — the shared state must
/// survive a re-projection switch.
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

    fn supports(&self, profile: &ProfileTag) -> bool {
        matches!(profile, ProfileTag::ExcelStrict)
    }

    fn mount(&self, ctx: StageContext) -> StageHandle {
        let shared = ctx.shared;
        let title = self.title;
        let testid = self.testid;
        let view = view! {
            <div class="calc-stage" data-testid=testid data-stage-title=title>
                <h2 class="calc-stage__title">{title}</h2>
                <p
                    class="calc-stage__continuity"
                    data-collapsed=move || shared.with(|state| state.collapsed_keys.len().to_string())
                    data-selection=move || shared.with(|state| state.selection_set.len().to_string())
                >
                    "Continuity — collapsed: "
                    {move || shared.with(|state| state.collapsed_keys.len())}
                    " · selection set: "
                    {move || shared.with(|state| state.selection_set.len())}
                </p>
            </div>
        }
        .into_any();
        StageHandle::new(view)
    }
}

/// The DEGRADE-mode workbench slot + the honest three-way outcome readout.
#[derive(Clone)]
struct CalcBridgeSurface {
    on_event: Callback<BridgeEvent>,
    seed_text: RwSignal<String>,
    rejections: RwSignal<Vec<GridEntryDiagnosticProjection>>,
    outcome: RwSignal<Option<CellOutcome>>,
    revision: RwSignal<usize>,
}

impl BridgeSurface for CalcBridgeSurface {
    fn mount(&self, _ctx: StageContext) -> AnyView {
        let on_event = self.on_event;
        let seed_text = self.seed_text;
        let rejections = self.rejections;
        let outcome = self.outcome;
        let revision = self.revision;
        view! {
            <div class="calc-bridge">
                {move || {
                    // Remount the degrade editor only on commit/revert (a
                    // revision bump) — never per keystroke — re-seeding from the
                    // last committed text and carrying the latest rejections.
                    revision.get();
                    let seed = seed_text.get_untracked();
                    let rej = rejections.get_untracked();
                    view! {
                        <FormulaBridgeDegrade text=seed rejections=rej on_event=on_event />
                    }
                }}
                {move || {
                    outcome
                        .get()
                        .map(|current| {
                            let label = current.label();
                            let detail = outcome_detail(&current);
                            view! {
                                <div
                                    class="calc-outcome"
                                    data-testid="calc-outcome"
                                    data-outcome=label
                                >
                                    <span class="calc-outcome__label">{label}</span>
                                    <span class="calc-outcome__detail">{detail}</span>
                                </div>
                            }
                        })
                }}
            </div>
        }
        .into_any()
    }
}

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

/// The DNA Calc app root.
#[component]
pub fn CalcApp(runtime: RuntimeContext) -> impl IntoView {
    let workspace = RwSignal::new(WorkspaceState::default());
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(0));
    let selection = RwSignal::new(SelectionState::default());
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());

    // The real host-core demo workbook, published into the workspace signal by
    // the workbook dispatcher (edits recalc dependents live).
    let workbook = Arc::new(
        WorkbookHostDispatcher::new_demo(workspace, latest_delta, selection, Some(shared))
            .expect("build the demo workbook"),
    );
    let inner: Arc<dyn Dispatcher> = workbook;
    let dispatch: Arc<dyn Dispatcher> = Arc::new(PersonaDispatcher { inner, shared });

    // Degrade-path signals.
    let text_buffer = RwSignal::new(String::new());
    let committed_text = RwSignal::new(String::new());
    let seed_text = RwSignal::new(String::new());
    let rejections = RwSignal::new(Vec::<GridEntryDiagnosticProjection>::new());
    let outcome = RwSignal::new(None::<CellOutcome>);
    let revision = RwSignal::new(0usize);

    let dispatch_for_events = dispatch.clone();
    let workspace_ro = workspace.read_only();
    let on_event = Callback::new(move |event: BridgeEvent| match event {
        // Verbatim text; the host classifies it — the skin never inspects `=`.
        BridgeEvent::TextEdited { text, .. } => text_buffer.set(text),
        BridgeEvent::CommitRequested => {
            let text = text_buffer.get_untracked();
            let grid = workspace_ro
                .with_untracked(|state| state.sheets.first().map(|sheet| sheet.grid_node_id.clone()));
            if let Some(grid) = grid {
                let receipt = dispatch_for_events.dispatch(enter_grid_cell_intent(grid, text.clone()));
                let resolved = interpret_receipt(&receipt);
                match &resolved {
                    CellOutcome::Rejected(diagnostics) => {
                        rejections.set(diagnostics.clone());
                        // Keep the rejected text so the user can fix it.
                        seed_text.set(text);
                    }
                    _ => {
                        rejections.set(Vec::new());
                        committed_text.set(text.clone());
                        seed_text.set(text);
                    }
                }
                outcome.set(Some(resolved));
                revision.update(|r| *r += 1);
            }
        }
        BridgeEvent::RevertRequested => {
            let committed = committed_text.get_untracked();
            text_buffer.set(committed.clone());
            seed_text.set(committed);
            rejections.set(Vec::new());
            outcome.set(None);
            revision.update(|r| *r += 1);
        }
        _ => {}
    });

    let mut composition = ShellComposition::calc(ProfileTag::ExcelStrict);
    composition.bridge_slot.surface = Some(Arc::new(CalcBridgeSurface {
        on_event,
        seed_text,
        rejections,
        outcome,
        revision,
    }));

    let stages = StageRegistry::new()
        .with_stage(Arc::new(StubStage {
            id: StageId::Sheet,
            title: "Sheet",
            testid: "calc-stage-sheet",
        }))
        .with_stage(Arc::new(StubStage {
            id: StageId::Model,
            title: "Model",
            testid: "calc-stage-model",
        }));

    view! {
        <style>{CALC_APP_CSS}</style>
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

const CALC_APP_CSS: &str = "\
.calc-stage{padding:var(--dna-gap-5);display:flex;flex-direction:column;gap:var(--dna-gap-3)}
.calc-stage__title{margin:0;font-size:16px}
.calc-stage__continuity{color:var(--dna-ink-2);font-size:13px}
.calc-bridge{display:flex;flex-direction:column;gap:var(--dna-gap-3)}
.calc-outcome{display:flex;gap:var(--dna-gap-3);align-items:baseline;padding:var(--dna-gap-2) var(--dna-gap-4);border-radius:var(--dna-radius-chip);background:var(--dna-paper-2)}
.calc-outcome__label{font-weight:600;text-transform:uppercase;letter-spacing:0.05em;font-size:11px;color:var(--dna-accent-ink)}
.calc-outcome__detail{font-size:12px;color:var(--dna-ink-2)}
";

//! The DNA Calc product composition — `ShellComposition::calc()` with the real
//! `dnacalc-stage-sheet::SheetStage` (S3.11) plus a remaining Model stub stage,
//! over the real `dnacalc-host-core` demo workbook (driven through
//! `dnatreecalc-host`'s `WorkbookHostDispatcher`), and the formula workbench in
//! DEGRADE mode editing one workbook cell via `EnterGridCell` with the
//! three-way outcome rendered honestly.
//!
//! W011 Wave 1.5 (dtc-j7n8.10): the shell's own document-lifecycle seam —
//! the command deck's `shell.open` / `shell.save` and the Ctrl+O / Ctrl+S
//! verbs — is wired to the host's `OpenXlsxBytes` / `SaveActiveXlsx`
//! commands through [`DocumentController`]. File dialogs live in the desktop
//! shell (`dnacalc-app-desktop`, reached over [`crate::shell_files`]); only
//! bytes cross into this crate, and skins never see a file API.

use std::sync::Arc;

use leptos::prelude::*;

use dnacalc_bridge::{BridgeEvent, FormulaBridgeDegrade};
use dnacalc_shell::{
    BridgeSurface, ProfileTag, RuntimeContext, Shell, ShellComposition, StageContext, StageHandle,
    StageId, StageRegistry, StageSurface,
};
use dnacalc_skin_ir::IntentReceipt;
use dnacalc_skin_ir::intent::{Dispatcher, WorkspaceDelta, WorkspaceIntent};
use dnacalc_skin_ir::keychord::SkinVerb;
use dnacalc_skin_ir::protocol::SkinShellIntent;
use dnacalc_skin_ir::selection::SelectionState;
use dnacalc_skin_ir::state::{SharedSkinState, SharedStateChange, SharedStateOrigin};
use dnacalc_skin_ir::workspace::{GridEntryDiagnosticProjection, WorkspaceState};
use dnacalc_skin_leptos::state_handles::SharedSkinStateHandle;
use dnacalc_stage_atlas::AtlasStage;
use dnacalc_stage_notebook::NotebookStage;
use dnacalc_stage_sheet::SheetStage;
use dnacalc_strand::{Density, Theme};
use dnatreecalc_host::app::WorkbookHostDispatcher;

use crate::adapter::{CellOutcome, enter_grid_cell_intent, interpret_receipt};
use crate::document::{DocumentController, FileVerb};

/// Wraps the workbook dispatcher to make persona switching real: `SetPersona`
/// is written back into `SharedSkinState.persona` on accept (SHELL_SPEC §4 —
/// the deck's persona marker mirrors that field and only changes when the host
/// applies it). Every other intent delegates to the workbook dispatcher, and
/// an accepted receipt that carries a change (an entry, a clear, a
/// defined-name edit — never a revision-inert select/interest receipt) marks
/// the document dirty for the mast / persistence projection (dtc-j7n8.10).
struct CalcDispatcher {
    inner: Arc<dyn Dispatcher>,
    shared: SharedSkinStateHandle,
    documents: DocumentController,
}

impl Dispatcher for CalcDispatcher {
    fn dispatch(&self, intent: WorkspaceIntent) -> IntentReceipt {
        if let WorkspaceIntent::SetPersona { persona } = &intent {
            self.shared.apply(
                SharedStateChange::SetPersona(*persona),
                SharedStateOrigin::Host,
            );
            return IntentReceipt::accepted();
        }
        let receipt = self.inner.dispatch(intent);
        if receipt.accepted && !receipt.delta.changes.is_empty() {
            self.documents.mark_dirty();
        }
        receipt
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

/// Whether this page can reach the desktop shell's file bridge (the Tauri
/// webview with `withGlobalTauri`). Never in the native compile, which has
/// no window at all.
#[must_use]
pub fn shell_file_bridge_available() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        crate::shell_files::bridge_available()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// Run one document-lifecycle verb the shell asked for. Every file picker is
/// async and lives in the desktop shell, so on wasm the flow is spawned:
/// pick -> bytes -> host command -> projection/status (Open), or host
/// command -> bytes -> pick -> write -> projection/status (Save). Where no
/// bridge exists the verb is answered with an honest status note and nothing
/// changes — never a silent no-op, never a fabricated success.
fn run_file_verb(documents: &DocumentController, verb: FileVerb) {
    if documents.bridge_available() {
        spawn_file_verb(documents, verb);
    } else {
        documents.note_bridge_unavailable(verb);
    }
}

/// wasm: hand the verb to the async shell flow.
#[cfg(target_arch = "wasm32")]
fn spawn_file_verb(documents: &DocumentController, verb: FileVerb) {
    let documents = documents.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match verb {
            FileVerb::Open => open_through_shell(&documents).await,
            FileVerb::Save => save_through_shell(&documents).await,
        }
    });
}

/// Native: there is no shell flow to hand a verb to (the native compile never
/// reports a bridge, so this is reachable only through a controller built with
/// `bridge_available = true` in a test) — answer honestly, never silently.
#[cfg(not(target_arch = "wasm32"))]
fn spawn_file_verb(documents: &DocumentController, verb: FileVerb) {
    documents.note_bridge_unavailable(verb);
}

/// Open: the shell's native dialog resolves the bytes, the host opens them.
#[cfg(target_arch = "wasm32")]
async fn open_through_shell(documents: &DocumentController) {
    match crate::shell_files::pick_xlsx_to_open().await {
        Ok(Some(file)) => {
            // The outcome (Opened / a typed refusal) is folded into the
            // projection + status by the controller.
            let _ = documents.open_bytes(file.bytes, file.name, Some(file.path));
        }
        Ok(None) => documents.note_cancelled(FileVerb::Open),
        Err(error) => documents.note_bridge_error(FileVerb::Open, &error),
    }
}

/// Save: the host produces the package bytes first (a refusal — the demo's
/// `NoBackingSource` — ends here with the status set), then the shell's
/// native dialog picks where they go and reports the write.
#[cfg(target_arch = "wasm32")]
async fn save_through_shell(documents: &DocumentController) {
    let crate::adapter::SaveOutcome::Saved { bytes, .. } = documents.save_to_bytes() else {
        return;
    };
    let suggested_name = documents.suggested_save_name();
    let suggested_directory = documents.suggested_directory();
    match crate::shell_files::pick_path_and_save_xlsx(
        &suggested_name,
        suggested_directory.as_deref(),
        &bytes,
    )
    .await
    {
        Ok(Some(saved)) => documents.mark_saved(saved.path, saved.name, saved.bytes_written),
        Ok(None) => documents.note_cancelled(FileVerb::Save),
        Err(error) => documents.note_bridge_error(FileVerb::Save, &error),
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
    // the workbook dispatcher (edits recalc dependents live). The demo stays
    // the default mount; Open replaces it with a real `.xlsx` through the same
    // dispatcher (W011).
    let workbook = Arc::new(
        WorkbookHostDispatcher::new_demo(workspace, latest_delta, selection, Some(shared))
            .expect("build the demo workbook"),
    );
    let documents = DocumentController::new(
        workbook.clone(),
        workspace.read_only(),
        shared,
        shell_file_bridge_available(),
    );
    let inner: Arc<dyn Dispatcher> = workbook;
    let dispatch: Arc<dyn Dispatcher> = Arc::new(CalcDispatcher {
        inner,
        shared,
        documents: documents.clone(),
    });

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
        // The app's single-formula bridge slot has no grid to walk, so the Tab-vs-
        // Enter `advance` is irrelevant here — commit either way.
        BridgeEvent::CommitRequested { .. } => {
            let text = text_buffer.get_untracked();
            let grid = workspace_ro.with_untracked(|state| {
                state.sheets.first().map(|sheet| sheet.grid_node_id.clone())
            });
            if let Some(grid) = grid {
                let receipt =
                    dispatch_for_events.dispatch(enter_grid_cell_intent(grid, text.clone()));
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

    // The shell's document-lifecycle seam (bead dtc-lfz.3): the deck's
    // `shell.open` / `shell.save` dispatch a `SkinShellIntent` here once the
    // controller's projection advertises the capability; Ctrl+O / Ctrl+S
    // arrive as forwarded verbs. Both routes run the same flow.
    let documents_for_intents = documents.clone();
    let on_shell_intent = Callback::new(move |intent: SkinShellIntent| match intent {
        SkinShellIntent::Open { .. } => run_file_verb(&documents_for_intents, FileVerb::Open),
        SkinShellIntent::Save | SkinShellIntent::SaveAs { .. } => {
            run_file_verb(&documents_for_intents, FileVerb::Save);
        }
        // Palette / active-document / recent-document intents are not part
        // of the deck's Save/Open channel; the Calc app wires none of them.
        SkinShellIntent::OpenCommandPalette
        | SkinShellIntent::CloseCommandPalette
        | SkinShellIntent::SetActiveDocument { .. }
        | SkinShellIntent::OpenRecent { .. } => {}
    });
    let documents_for_verbs = documents.clone();
    let on_shell_verb = Callback::new(move |verb: SkinVerb| {
        if let Some(file_verb) = FileVerb::from_shell_verb(verb) {
            run_file_verb(&documents_for_verbs, file_verb);
        }
    });
    let persistence = documents.persistence();
    let document_status = documents.status();
    let document_name = documents.document_name();
    let bridge_available = documents.bridge_available();

    let mut composition = ShellComposition::calc(ProfileTag::ExcelStrict);
    composition.bridge_slot.surface = Some(Arc::new(CalcBridgeSurface {
        on_event,
        seed_text,
        rejections,
        outcome,
        revision,
    }));

    let stages = StageRegistry::new()
        .with_stage(Arc::new(SheetStage::new()))
        .with_stage(Arc::new(StubStage {
            id: StageId::Model,
            title: "Model",
            testid: "calc-stage-model",
        }))
        .with_stage(Arc::new(NotebookStage::new()))
        .with_stage(Arc::new(AtlasStage::new()));

    view! {
        <style>{CALC_APP_CSS}</style>
        <div class="calc-app">
            // The document line (dtc-j7n8.10): which `.xlsx` is active (or the
            // demo), whether this runtime can open/save at all, and the last
            // lifecycle outcome verbatim — the click-through's readout.
            <div
                class="calc-document"
                data-testid="calc-document"
                data-document-status=move || {
                    document_status
                        .with(|status| status.as_ref().map_or("none", |status| status.label))
                }
                data-file-bridge=if bridge_available { "available" } else { "unavailable" }
            >
                <span class="calc-document__name">
                    {move || {
                        document_name
                            .with(|name| {
                                name.clone().unwrap_or_else(|| "demo workbook (in-memory)".to_string())
                            })
                    }}
                </span>
                <span class="calc-document__detail">
                    {move || {
                        document_status
                            .with(|status| {
                                status.as_ref().map_or_else(
                                    || {
                                        if bridge_available {
                                            "Open (Ctrl+O) / Save (Ctrl+S) through the desktop shell".to_string()
                                        } else {
                                            "Open/Save need the desktop shell's file bridge; none in this runtime".to_string()
                                        }
                                    },
                                    |status| status.detail.clone(),
                                )
                            })
                    }}
                </span>
            </div>
            <div class="calc-app__shell">
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
                    host_persistence=persistence
                    on_shell_intent=Some(on_shell_intent)
                    on_shell_verb=Some(on_shell_verb)
                />
            </div>
        </div>
    }
}

const CALC_APP_CSS: &str = "\
.calc-app{display:flex;flex-direction:column;height:100%;min-height:0}
.calc-app__shell{flex:1 1 auto;min-height:0}
.calc-document{flex:none;display:flex;gap:12px;align-items:baseline;padding:4px 12px;font:12px system-ui,sans-serif;background:#f4f4f2;color:#333;border-bottom:1px solid #ddd}
.calc-document__name{font-weight:600}
.calc-document__detail{color:#555;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.calc-stage{padding:var(--dna-gap-5);display:flex;flex-direction:column;gap:var(--dna-gap-3)}
.calc-stage__title{margin:0;font-size:16px}
.calc-stage__continuity{color:var(--dna-ink-2);font-size:13px}
.calc-bridge{display:flex;flex-direction:column;gap:var(--dna-gap-3)}
.calc-outcome{display:flex;gap:var(--dna-gap-3);align-items:baseline;padding:var(--dna-gap-2) var(--dna-gap-4);border-radius:var(--dna-radius-chip);background:var(--dna-paper-2)}
.calc-outcome__label{font-weight:600;text-transform:uppercase;letter-spacing:0.05em;font-size:11px;color:var(--dna-accent-ink)}
.calc-outcome__detail{font-size:12px;color:var(--dna-ink-2)}
";

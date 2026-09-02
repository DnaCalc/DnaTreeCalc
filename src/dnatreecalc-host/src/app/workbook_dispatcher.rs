//! `WorkbookHostDispatcher` — the app-side dispatcher that drives a strict-Excel
//! workbook through the clean `dnacalc-host-core` spine (Phase 0 keystone).
//!
//! This is the *parallel* workbook path: where [`HostDispatcher`](super::dispatcher::HostDispatcher)
//! routes intents into the legacy `TreeWorkspaceSession`, this routes the
//! workbook-family intents into a [`dnacalc_host_core::DocumentSession`] and
//! republishes that session's full [`WorkspaceState`] snapshot after every
//! accepted mutation — reusing the "replace the whole workspace signal" model
//! the tree dispatcher already uses, so dependent cells recalc live (edit A1 →
//! B1 updates) with no delta-application machinery.
//!
//! ## Deltas vs the snapshot (W011, dtc-j7n8.18)
//!
//! An accepted entry receipt now carries, beside its `GridCellEntered` hint,
//! the edited sheet's complete `GridChanged` plus one per other sheet the
//! edit's cross-sheet recalc moved, so a retained mirror could patch every
//! changed sheet in place. This dispatcher deliberately KEEPS the snapshot
//! republish: host-core stamps `from_seq`/`to_seq` = 0 on every receipt (it
//! owns no projection-sequence authority), while the workspace signal carries
//! this dispatcher's own monotonic `projection_seq` — so
//! `session_channel::apply_delta` would report a sequence gap on every
//! receipt, and the snapshot stays the one authoritative signal path. The
//! receipt's delta is published on the delta signal as-is: a harmless double
//! publish, because the patch and the snapshot come out of the same host-core
//! recipe (`WorkbookSession::grid_projection_from_view`), which the app test
//! `app_opens_fixture_edits_and_saves_through_commands` asserts cell for cell.
//! Trusting deltas here would mean stamping sequences on this side of the
//! seam — a later change, not a side effect of the receipt growing a patch.
//!
//! ## Commands (W011, dtc-j7n8.8)
//!
//! `WorkspaceIntent` mutates the *open model*; [`HostCommand`] manages
//! *documents and files*. [`WorkbookHostDispatcher::execute_host_command`] is
//! the one door a shell has to the command surface — open `.xlsx` bytes
//! (`OpenXlsxBytes`, replacing the session's document and republishing the
//! snapshot) and save the active workbook back to bytes (`SaveActiveXlsx`,
//! handing the bytes to the caller untouched). Skins never call OxDoc, OxCalc
//! or file APIs; the shell owns file I/O (dialogs, drag-drop, fetch) and hands
//! a plain byte buffer in or takes one out. The demo mount ([`Self::new_demo`])
//! is untouched by this: it is an in-memory workbook with no backing source, so
//! a save on it is host-core's typed `NoBackingSource` refusal, never a panic.
//!
//! ## `!Send` handling
//!
//! [`Dispatcher`] is `Send + Sync`, but a `DocumentSession` is **neither**
//! (it transitively holds `Rc<RichValue>` inside the engine's `CalcValue`).
//! So — exactly as the tree dispatcher keeps its `!Send` sessions in a
//! `thread_local` and holds only a numeric id — the session lives in
//! [`WORKBOOK_SESSIONS`] keyed by id, and this dispatcher carries only that id
//! plus the (`Send + Sync`) Leptos signals. On the single wasm main thread the
//! id always resolves on the owning thread.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use dnacalc_host_core::{
    DocumentSession, HostCommand, HostCommandError, HostCommandOutcome, WorkbookSession,
    WorkbookSessionError, XLSX_WORKSPACE_ID, build_demo_workbook,
};
use dnatreecalc_skin_framework::{
    Dispatcher, IntentReceipt, SelectionState, SharedSkinStateHandle, SharedStateChange,
    SharedStateOrigin, WorkspaceDelta, WorkspaceIntent, WorkspaceState,
};
use leptos::prelude::*;

thread_local! {
    /// The `!Send` workbook sessions, owned per-thread and addressed by id so
    /// the `Send + Sync` dispatcher never holds one as a field.
    static WORKBOOK_SESSIONS: RefCell<BTreeMap<u64, DocumentSession>> =
        const { RefCell::new(BTreeMap::new()) };
}

static NEXT_WORKBOOK_SESSION_ID: AtomicU64 = AtomicU64::new(1);

/// A [`HostCommand`] this dispatcher could not carry through to a successful
/// outcome. The session's own refusal travels as host-core's typed
/// [`HostCommandError`] **untouched** (OxDoc's `XlsxError`, the engine's
/// error, `NoBackingSource`, `UnsupportedByModel` — all data, never a
/// formatted string); the one dispatcher-level arm is the command analogue of
/// the `GenericEngineRejection` receipt [`Dispatcher::dispatch`] answers with
/// when the session id does not resolve on the calling thread.
// `large_enum_variant`: `Command` wraps `HostCommandError` by value (which
// wraps the engine error by value — host-core's documented convention) and
// `SessionUnavailable` is one `u64`. Boxing would break the `#[from]`/`?`
// conversion `execute_host_command` relies on for no caller benefit (one host
// call per command, not a hot loop). Kept by-value for consistency.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, thiserror::Error)]
pub enum WorkbookHostCommandError {
    /// The document session rejected the command — host-core's typed error,
    /// passed through as-is.
    #[error(transparent)]
    Command(#[from] HostCommandError),
    /// The dispatcher's session id did not resolve on this thread (never on
    /// the single wasm main thread that created it).
    #[error("workbook session {session_id} did not resolve on this thread")]
    SessionUnavailable { session_id: u64 },
}

/// A `Dispatcher` over one host-core workbook document, publishing its full
/// projection snapshot into the shared workspace signal.
pub struct WorkbookHostDispatcher {
    session_id: u64,
    workspace: RwSignal<WorkspaceState>,
    latest_delta: RwSignal<WorkspaceDelta>,
    selection: RwSignal<SelectionState>,
    shared: Option<SharedSkinStateHandle>,
    next_projection_seq: AtomicU64,
}

// `result_large_err`: `WorkbookSessionError` / `WorkbookHostCommandError` are
// returned by value to match the crate-wide convention (host-core's own
// `WorkbookSession` and the sibling `TreeWorkspaceSessionError`); these are
// one-shot mount / open / save calls, not a hot `Result`-returning loop.
#[allow(clippy::result_large_err)]
impl WorkbookHostDispatcher {
    /// Adopt a host-core [`DocumentSession`], seed the workspace signal with its
    /// initial snapshot, and select the first sheet's grid so a lens has a
    /// starting context.
    #[must_use]
    pub fn new(
        document: DocumentSession,
        workspace: RwSignal<WorkspaceState>,
        latest_delta: RwSignal<WorkspaceDelta>,
        selection: RwSignal<SelectionState>,
        shared: Option<SharedSkinStateHandle>,
    ) -> Self {
        let session_id = NEXT_WORKBOOK_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        WORKBOOK_SESSIONS.with(|sessions| {
            sessions.borrow_mut().insert(session_id, document);
        });
        let dispatcher = Self {
            session_id,
            workspace,
            latest_delta,
            selection,
            shared,
            next_projection_seq: AtomicU64::new(1),
        };
        let initial = dispatcher.publish_snapshot();
        dispatcher.select_first_grid(&initial);
        dispatcher
    }

    /// Build the Phase-0 demonstration workbook (two sheets, live formulas) and
    /// wrap it in a dispatcher. The `?wb=1` mount path uses this.
    pub fn new_demo(
        workspace: RwSignal<WorkspaceState>,
        latest_delta: RwSignal<WorkspaceDelta>,
        selection: RwSignal<SelectionState>,
        shared: Option<SharedSkinStateHandle>,
    ) -> Result<Self, WorkbookSessionError> {
        let document = DocumentSession::Workbook(build_demo_workbook()?);
        Ok(Self::new(
            document,
            workspace,
            latest_delta,
            selection,
            shared,
        ))
    }

    /// Open `.xlsx` bytes through OxDoc, ingest them into the engine
    /// ([`WorkbookSession::open_xlsx_bytes`], W011) and wrap the opened
    /// workbook in a dispatcher — the mount entry point for a shell that
    /// already holds a document's bytes (a file dialog, drag-drop, a fetch).
    /// `name` is the user-facing document name the bytes arrived under.
    ///
    /// OxDoc's rejection of the bytes (a corrupt zip, a missing part, an XML
    /// error) comes back as [`WorkbookSessionError::Xlsx`] and the engine's
    /// rejection of the stream as [`WorkbookSessionError::OxCalc`] — typed
    /// data, never a panic; nothing is mounted on failure.
    pub fn new_from_xlsx_bytes(
        bytes: &[u8],
        name: Option<String>,
        workspace: RwSignal<WorkspaceState>,
        latest_delta: RwSignal<WorkspaceDelta>,
        selection: RwSignal<SelectionState>,
        shared: Option<SharedSkinStateHandle>,
    ) -> Result<Self, WorkbookSessionError> {
        let session = WorkbookSession::open_xlsx_bytes(XLSX_WORKSPACE_ID, bytes, name)?;
        Ok(Self::new(
            DocumentSession::Workbook(session),
            workspace,
            latest_delta,
            selection,
            shared,
        ))
    }

    /// Execute one [`HostCommand`] against the owned document session and
    /// keep the signals honest about the result:
    ///
    /// - [`HostCommandOutcome::Opened`] — the session now holds the opened
    ///   workbook (the previous document dropped inside host-core), so the
    ///   full snapshot is republished, the caret moves to the loaded
    ///   workbook's first grid (the old grid ids no longer exist), and a
    ///   revision-inert delta carrying the new projection sequence is
    ///   published so delta observers see the swap.
    /// - [`HostCommandOutcome::Saved`] — the bytes and OxDoc's save ledger are
    ///   passed through untouched: the caller owns the bytes (the shell
    ///   persists them wherever they go), and host-core guarantees the save
    ///   neither replaced nor mutated the session, so nothing is republished.
    /// - [`HostCommandOutcome::Dispatched`] — exactly the
    ///   [`Dispatcher::dispatch`] behaviour: an accepted receipt republishes
    ///   the snapshot and the receipt's delta; a rejected one changes nothing.
    ///
    /// On failure the session is exactly as host-core left it (an open that
    /// fails leaves the previous document in place; a refused save mutates
    /// nothing) and the signals are not touched.
    pub fn execute_host_command(
        &self,
        command: HostCommand,
    ) -> Result<HostCommandOutcome, WorkbookHostCommandError> {
        // The `WORKBOOK_SESSIONS` borrow ends with this closure; every
        // `publish_*` below re-borrows the map and so must run OUTSIDE it.
        let outcome = self
            .with_session(|session| session.execute(command))
            .ok_or(WorkbookHostCommandError::SessionUnavailable {
                session_id: self.session_id,
            })??;
        match &outcome {
            HostCommandOutcome::Opened { .. } => {
                let state = self.publish_snapshot();
                self.select_first_grid(&state);
                self.publish_unchanged_delta();
            }
            HostCommandOutcome::Dispatched(receipt) => self.publish_after_receipt(receipt),
            HostCommandOutcome::Saved { .. } => {}
            // `HostCommandOutcome` is `#[non_exhaustive]`: an outcome this
            // dispatcher does not know yet publishes nothing (the session
            // may have changed, but no arm has claimed what to show) and is
            // still returned to the caller, so wiring it is a visible follow-up
            // rather than a silent one.
            _ => {}
        }
        Ok(outcome)
    }

    /// Run a closure against the owned session, or `None` if the id does not
    /// resolve on this thread (never happens on the single wasm main thread).
    fn with_session<R>(&self, f: impl FnOnce(&mut DocumentSession) -> R) -> Option<R> {
        WORKBOOK_SESSIONS.with(|sessions| sessions.borrow_mut().get_mut(&self.session_id).map(f))
    }

    /// Re-read the session's full projection, stamp a fresh projection sequence,
    /// and push it into the workspace signal. Returns the published state.
    fn publish_snapshot(&self) -> WorkspaceState {
        let mut state = self
            .with_session(|session| session.snapshot())
            .unwrap_or_default();
        state.projection_seq = self.next_projection_seq.fetch_add(1, Ordering::Relaxed);
        self.workspace.set(state.clone());
        state
    }

    /// A revision-inert delta carrying the current projection sequence — the
    /// selection/interest analogue of the tree dispatcher's
    /// `publish_unchanged_delta`.
    fn publish_unchanged_delta(&self) -> WorkspaceDelta {
        let seq = self.workspace.get_untracked().projection_seq;
        let delta = WorkspaceDelta::unchanged(seq);
        self.latest_delta.set(delta.clone());
        delta
    }

    /// Land the caret on the first grid-backed sheet of `state` so the
    /// Sheet/Workbook lens opens with a concrete selection rather than empty
    /// (used on mount and after a document swap).
    fn select_first_grid(&self, state: &WorkspaceState) {
        if let Some(first_grid) = state.grids.keys().next().cloned() {
            self.selection
                .set(SelectionState::with_primary(Some(first_grid)));
        }
    }

    /// After a model intent answered by the session: an accepted receipt
    /// republishes the full snapshot (dependents recalc live in every open
    /// lens) and the receipt's own delta; a rejected receipt publishes
    /// nothing (host-core guarantees no mutation on that path). The delta's
    /// `GridChanged`s (dtc-j7n8.18: the edited sheet's, and any peer sheet
    /// the edit moved) are NOT applied onto the workspace signal — see the
    /// module doc: host-core stamps no sequence, the snapshot is the
    /// authoritative path, and the delta rides alongside for delta observers.
    fn publish_after_receipt(&self, receipt: &IntentReceipt) {
        if receipt.accepted {
            self.publish_snapshot();
            self.latest_delta.set(receipt.delta.clone());
        }
    }
}

impl Dispatcher for WorkbookHostDispatcher {
    fn dispatch(&self, intent: WorkspaceIntent) -> IntentReceipt {
        match intent {
            // Selection is UI/session state (SKINS.md §2.5): route to the
            // signal, no engine call, revision-inert.
            WorkspaceIntent::SelectNode(target) => {
                self.selection.set(SelectionState::with_primary(target));
                IntentReceipt::accepted().with_delta(self.publish_unchanged_delta())
            }
            WorkspaceIntent::SelectNodes { keys, anchor } => {
                if let Some(shared) = &self.shared {
                    shared.apply(
                        SharedStateChange::SetSelectionSet(keys),
                        SharedStateOrigin::Host,
                    );
                    shared.apply(
                        SharedStateChange::SetSelectionAnchor(anchor),
                        SharedStateOrigin::Host,
                    );
                }
                IntentReceipt::accepted().with_delta(self.publish_unchanged_delta())
            }
            // Interest registration: the workbook snapshot already projects the
            // full populated bounding box of each sheet (the demo is small), so
            // there is no window to narrow yet — accept without a re-publish.
            // (Windowed interest becomes load-bearing with large real `.xlsx`
            // sheets in the file-I/O phase.)
            WorkspaceIntent::SetGridInterest { .. } => {
                IntentReceipt::accepted().with_delta(self.publish_unchanged_delta())
            }
            // Every model intent (EnterGridCell/ClearGridCell, the defined-name
            // family, SetCalcMode/Recalculate, and anything the workbook family
            // does not support → a typed UnsupportedByModel receipt) goes to the
            // host-core session. On acceptance, republish the full snapshot so
            // dependents recalc live in every open lens.
            intent => {
                let receipt = self
                    .with_session(|session| session.dispatch(intent))
                    .unwrap_or_else(|| {
                        IntentReceipt::rejected(
                            dnatreecalc_skin_framework::IntentError::GenericEngineRejection {
                                debug: "workbook session id did not resolve on this thread"
                                    .to_string(),
                            },
                        )
                    });
                self.publish_after_receipt(&receipt);
                receipt
            }
        }
    }
}

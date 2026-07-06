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

use dnacalc_host_core::{DocumentSession, WorkbookSessionError, build_demo_workbook};
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
        // Land the caret on the first grid-backed sheet so the Sheet/Workbook
        // lens opens with a concrete selection rather than empty.
        if let Some(first_grid) = initial.grids.keys().next().cloned() {
            dispatcher
                .selection
                .set(SelectionState::with_primary(Some(first_grid)));
        }
        dispatcher
    }

    /// Build the Phase-0 demonstration workbook (two sheets, live formulas) and
    /// wrap it in a dispatcher. The `?wb=1` mount path uses this.
    // `result_large_err`: `WorkbookSessionError` is returned by value to match
    // the crate-wide convention (host-core's own `WorkbookSession` and the
    // sibling `TreeWorkspaceSessionError`); this is a one-shot mount call, not a
    // hot `Result`-returning loop.
    #[allow(clippy::result_large_err)]
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
                if receipt.accepted {
                    self.publish_snapshot();
                    self.latest_delta.set(receipt.delta.clone());
                }
                receipt
            }
        }
    }
}

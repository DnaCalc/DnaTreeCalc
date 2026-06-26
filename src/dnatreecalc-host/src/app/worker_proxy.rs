//! The main-thread dispatcher **proxy** (B.2.2–B.2.4).
//!
//! When the `TreeWorkspaceSession` runs off the UI thread (a wasm web worker,
//! or a native thread), the main thread keeps a lightweight proxy: it owns the
//! projection mirror, hands intents to the session through a
//! [`SessionExecutor`], and applies the [`SessionResponse`]s back. The proxy is
//! the home of the worker semantics that are independent of the transport:
//!
//! - **seq + supersede-discard** (B.2.2 cancellation): every intent is stamped
//!   with a monotonic seq; a response that arrives out of order — a superseded
//!   run's late result — is dropped rather than applied over fresher state.
//! - **pending + coalescing** (B.2.4 backpressure): while a run is in flight
//!   incoming intents are parked, and deferred content edits coalesce per node
//!   so a keystroke burst collapses to the latest value.
//! - **mirror apply / resync** (B.2.3): a response carries a delta to apply to
//!   the retained mirror, or a full snapshot when the delta is not applicable
//!   or the sequence has gapped.
//! - **frame telemetry** (B.2.4): each applied response emits a [`FrameMetric`].
//!
//! The core here is pure (no Leptos, no web-sys) and unit-tested against a
//! scripted executor; the live host wraps it with the Leptos signals and an
//! in-process or `postMessage` transport.

use std::sync::{Arc, Mutex};

use dnatreecalc_skin_framework::{
    Dispatcher, FrameMetric, IntentEnvelope, IntentReceipt, PendingIntentQueue, PendingRun,
    ResyncReason, SelectionState, SessionResponse, WorkspaceDelta, WorkspaceIntent, WorkspaceState,
    apply_delta,
};
use leptos::prelude::*;

use super::dispatcher::HostDispatcher;
use super::session::TreeWorkspaceSession;

/// The session, behind whatever transport carries an envelope to it: an
/// in-process call (native / tests) or a `postMessage` round-trip (wasm
/// worker). The proxy only sees this trait.
pub trait SessionExecutor: Send + Sync {
    /// Run the envelope against the session and return its response. For an
    /// asynchronous transport this is the send half; the response is delivered
    /// to the proxy separately via [`WorkerProxyCore::deliver`].
    fn execute(&self, envelope: IntentEnvelope) -> SessionResponse;
}

/// What [`WorkerProxyCore::submit`] decided: the receipt to hand the caller
/// now, and the envelope to actually send (absent when the intent was parked
/// behind an in-flight run).
#[derive(Debug, Clone, PartialEq)]
pub struct SubmitDecision {
    pub receipt: IntentReceipt,
    pub to_send: Option<IntentEnvelope>,
}

/// What [`WorkerProxyCore::deliver`] did with a response: whether it was
/// applied (vs discarded as superseded), the metric to emit, and the next
/// parked envelope to send now that the run completed.
#[derive(Debug, Clone, PartialEq)]
pub struct DeliverOutcome {
    pub applied: bool,
    pub resynced: bool,
    pub metric: Option<FrameMetric>,
    pub next: Option<IntentEnvelope>,
    /// Set when a delta could not be applied and the proxy needs the executor
    /// to send a fresh snapshot (the caller re-requests via a recalculation).
    pub resync_reason: Option<ResyncReason>,
    /// The selection the intent produced (MIXED structural intents), for the
    /// host wrapper to mirror into the main-thread selection signal. `None`
    /// means the intent did not change selection — leave it alone.
    pub selection: Option<SelectionState>,
}

/// The transport-agnostic proxy state machine.
pub struct WorkerProxyCore {
    mirror: WorkspaceState,
    next_seq: u64,
    next_token: u64,
    highest_delivered_seq: u64,
    pending: Option<PendingRun>,
    queue: PendingIntentQueue,
}

impl WorkerProxyCore {
    /// Start from an initial snapshot (the session's first projection).
    #[must_use]
    pub fn new(initial: WorkspaceState) -> Self {
        Self {
            mirror: initial,
            next_seq: 1,
            next_token: 1,
            // Intent seq is its own monotonic counter (responses echo it),
            // distinct from the projection_seq the deltas carry. No intent has
            // been delivered yet, so nothing has been superseded.
            highest_delivered_seq: 0,
            pending: None,
            queue: PendingIntentQueue::new(),
        }
    }

    #[must_use]
    pub fn mirror(&self) -> &WorkspaceState {
        &self.mirror
    }

    #[must_use]
    pub fn pending(&self) -> Option<PendingRun> {
        self.pending
    }

    /// Stamp `intent` with the next seq and decide whether to send it now or
    /// park it behind the in-flight run. Parked deferred edits coalesce per
    /// node. The returned receipt is accepted-and-pending — the authoritative
    /// outcome arrives with the response.
    pub fn submit(&mut self, intent: WorkspaceIntent) -> SubmitDecision {
        let seq = self.next_seq;
        self.next_seq += 1;

        if self.pending.is_some() {
            self.queue.enqueue(seq, intent);
            return SubmitDecision {
                receipt: IntentReceipt::accepted(),
                to_send: None,
            };
        }

        self.begin_run();
        SubmitDecision {
            receipt: IntentReceipt::accepted(),
            to_send: Some(IntentEnvelope::new(seq, intent)),
        }
    }

    /// Apply a response from the executor. Discards a superseded (out-of-order)
    /// response; otherwise applies the delta to the mirror — or resets it from
    /// the snapshot — clears the in-flight run, and returns the next parked
    /// envelope to send.
    pub fn deliver(&mut self, response: SessionResponse, elapsed_micros: u64) -> DeliverOutcome {
        // A response older than the newest one we have already applied is a
        // superseded run's late arrival — discard it, but the run it belonged
        // to is no longer pending.
        if response.seq < self.highest_delivered_seq {
            self.pending = None;
            return DeliverOutcome {
                applied: false,
                resynced: false,
                metric: None,
                next: self.dispatch_next(),
                resync_reason: None,
                selection: None,
            };
        }
        let SessionResponse {
            seq,
            receipt,
            snapshot,
            selection,
        } = response;
        self.highest_delivered_seq = seq;
        self.pending = None;

        let mut resynced = false;
        let mut resync_reason = None;
        if let Some(snapshot) = snapshot {
            self.mirror = snapshot;
            resynced = true;
        } else if let Err(reason) = apply_delta(&mut self.mirror, &receipt.delta) {
            // The executor judged the delta applicable but our mirror has
            // gapped — surface it so the caller re-requests a snapshot.
            resync_reason = Some(reason);
        }

        let metric = FrameMetric {
            intent_seq: seq,
            dispatch_to_apply_micros: elapsed_micros,
            coalesced: self.queue.coalesced_count(),
            resynced,
        };
        self.queue.reset_coalesced();

        DeliverOutcome {
            applied: true,
            resynced,
            metric: Some(metric),
            next: self.dispatch_next(),
            resync_reason,
            selection,
        }
    }

    /// Pop the next parked envelope (if any) and mark a run in flight for it.
    fn dispatch_next(&mut self) -> Option<IntentEnvelope> {
        if self.pending.is_some() {
            return None;
        }
        let mut drained = self.queue.drain();
        if drained.is_empty() {
            return None;
        }
        // Send the oldest parked intent; re-park the rest so each run is
        // serialized (one in-flight at a time).
        let (seq, intent) = drained.remove(0);
        for (parked_seq, parked_intent) in drained {
            self.queue.enqueue(parked_seq, parked_intent);
        }
        self.begin_run();
        Some(IntentEnvelope::new(seq, intent))
    }

    fn begin_run(&mut self) {
        let token = self.next_token;
        self.next_token += 1;
        self.pending = Some(PendingRun {
            token,
            started_value_epoch: self.mirror.revision.value_epoch,
        });
    }
}

/// A [`SessionExecutor`] backed by a real [`HostDispatcher`] running in
/// **executor mode**: it owns the session and the engine, runs the intent, and
/// reports the resulting snapshot/delta plus the selection the intent produced.
///
/// This is exactly the code the wasm worker entry runs — the worker constructs
/// one of these over the session it owns and calls [`Self::execute`] for each
/// envelope. The Leptos signals it holds are **throwaway**: the dispatcher
/// writes them (so `publish_projection_state` computes the delta and the
/// MIXED-intent selection lands), but the main-thread proxy owns the *real* UI
/// signals. Shared state, undo/redo, persistence, the persona gate, and intent
/// recording all stay on the main thread (the dispatcher here has no shared
/// handle), so the executor is pure engine.
pub struct HostSessionExecutor {
    dispatcher: HostDispatcher,
    workspace: RwSignal<WorkspaceState>,
    selection: RwSignal<SelectionState>,
}

impl HostSessionExecutor {
    /// Build an executor over `session`. Must be called within a Leptos
    /// reactive owner (the worker establishes one at startup; tests use
    /// `Owner::new`).
    #[must_use]
    pub fn new(session: Arc<Mutex<TreeWorkspaceSession>>) -> Self {
        let initial = session
            .lock()
            .expect("workspace session mutex poisoned")
            .workspace_state()
            .expect("workspace session must project");
        let selection = RwSignal::new(SelectionState::default());
        let workspace = RwSignal::new(initial.clone());
        let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(initial.projection_seq));
        let dispatcher = HostDispatcher::with_session(selection, workspace, latest_delta, session);
        Self {
            dispatcher,
            workspace,
            selection,
        }
    }

    /// The session's current projection — the proxy's initial mirror.
    #[must_use]
    pub fn snapshot(&self) -> WorkspaceState {
        self.workspace.get_untracked()
    }
}

impl SessionExecutor for HostSessionExecutor {
    fn execute(&self, envelope: IntentEnvelope) -> SessionResponse {
        let selection_before = self.selection.get_untracked();
        let receipt = self.dispatcher.dispatch(envelope.intent);
        let state = self.workspace.get_untracked();
        let selection_after = self.selection.get_untracked();
        // Report the selection only when the intent actually changed it — that
        // captures exactly the MIXED structural intents; everything else
        // leaves the main-thread selection untouched.
        let selection = (selection_before != selection_after).then_some(selection_after);
        SessionResponse::for_receipt(envelope.seq, receipt, &state, selection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{WorkspaceFixture, WorkspaceModel};
    use dnatreecalc_skin_framework::{
        InitialNodeContentProjection, NodeId, WorkspaceDelta, WorkspaceDeltaChange,
    };

    fn snapshot_at(seq: u64) -> WorkspaceState {
        WorkspaceState {
            projection_seq: seq,
            ..WorkspaceState::default()
        }
    }

    fn snapshot_response(seq: u64, projection_seq: u64) -> SessionResponse {
        SessionResponse {
            seq,
            receipt: IntentReceipt::accepted()
                .with_delta(WorkspaceDelta::unchanged(projection_seq)),
            snapshot: Some(snapshot_at(projection_seq)),
            selection: None,
        }
    }

    fn edit(node: &str) -> WorkspaceIntent {
        WorkspaceIntent::EditContentDeferred {
            node: NodeId::new(node),
            content: "1".to_string(),
        }
    }

    #[test]
    fn submit_sends_first_intent_and_parks_the_rest_behind_the_run() {
        let mut core = WorkerProxyCore::new(snapshot_at(0));
        let first = core.submit(WorkspaceIntent::Recalculate);
        assert!(first.to_send.is_some(), "first intent goes out immediately");
        assert!(core.pending().is_some());

        // A second intent while pending is parked, not sent.
        let second = core.submit(WorkspaceIntent::Recalculate);
        assert!(second.to_send.is_none());
        assert!(second.receipt.accepted);
    }

    #[test]
    fn deliver_applies_a_snapshot_and_releases_the_next_parked_intent() {
        let mut core = WorkerProxyCore::new(snapshot_at(0));
        let first = core.submit(WorkspaceIntent::Recalculate).to_send.unwrap();
        core.submit(WorkspaceIntent::Recalculate); // parked

        let outcome = core.deliver(snapshot_response(first.seq, 1), 250);
        assert!(outcome.applied);
        assert!(outcome.resynced);
        assert_eq!(core.mirror().projection_seq, 1);
        assert_eq!(outcome.metric.unwrap().dispatch_to_apply_micros, 250);
        // The parked recalc is now in flight.
        assert!(outcome.next.is_some());
        assert!(core.pending().is_some());
    }

    #[test]
    fn a_superseded_response_is_discarded_not_applied_over_fresher_state() {
        let mut core = WorkerProxyCore::new(snapshot_at(0));
        let first = core.submit(WorkspaceIntent::Recalculate).to_send.unwrap();
        // Apply a newer response first (seq advances the high-water mark)...
        let _ = core.deliver(snapshot_response(first.seq, 5), 10);
        assert_eq!(core.mirror().projection_seq, 5);
        // ...then a stale, lower-seq response arrives late and is dropped.
        let stale = core.deliver(snapshot_response(first.seq.saturating_sub(1).max(0), 2), 10);
        assert!(!stale.applied, "superseded response must not apply");
        assert_eq!(
            core.mirror().projection_seq,
            5,
            "mirror keeps the fresher state"
        );
    }

    #[test]
    fn deferred_edits_coalesce_per_node_while_a_run_is_pending() {
        let mut core = WorkerProxyCore::new(snapshot_at(0));
        let first = core.submit(WorkspaceIntent::Recalculate).to_send.unwrap();
        // Three deferred edits to the same node land while the run is pending;
        // they coalesce to one parked edit.
        core.submit(edit("A"));
        core.submit(edit("A"));
        core.submit(edit("A"));

        let outcome = core.deliver(snapshot_response(first.seq, 1), 0);
        // The coalesced count is reported in the metric, and exactly one edit
        // is released next.
        assert_eq!(outcome.metric.unwrap().coalesced, 2);
        let next = outcome.next.expect("the coalesced edit is released");
        assert!(matches!(
            next.intent,
            WorkspaceIntent::EditContentDeferred { .. }
        ));
        // Nothing else is parked.
        let drained = core.deliver(snapshot_response(next.seq, 2), 0);
        assert!(drained.next.is_none());
    }

    #[test]
    fn deliver_applies_a_delta_when_no_snapshot_is_sent() {
        let mut core = WorkerProxyCore::new(snapshot_at(3));
        let first = core.submit(WorkspaceIntent::Recalculate).to_send.unwrap();
        // A delta-only response (a VerifiedClean recalc carries only CalcRun,
        // which the mirror can apply without a snapshot).
        let delta = WorkspaceDelta {
            from_seq: 3,
            to_seq: 4,
            changes: vec![WorkspaceDeltaChange::ClipboardChanged(None)],
        };
        let response = SessionResponse {
            seq: first.seq,
            receipt: IntentReceipt::accepted().with_delta(delta),
            snapshot: None,
            selection: None,
        };
        let outcome = core.deliver(response, 0);
        assert!(outcome.applied);
        assert!(!outcome.resynced, "a delta-only response does not resync");
        assert_eq!(core.mirror().projection_seq, 4);
        assert!(outcome.resync_reason.is_none());
    }

    #[test]
    fn deliver_flags_a_resync_when_a_delta_cannot_apply() {
        let mut core = WorkerProxyCore::new(snapshot_at(3));
        let first = core.submit(WorkspaceIntent::Recalculate).to_send.unwrap();
        // The executor sent delta-only but the mirror has gapped (from_seq 9 !=
        // mirror 3): the proxy keeps the mirror and asks for a snapshot.
        let delta = WorkspaceDelta {
            from_seq: 9,
            to_seq: 10,
            changes: vec![WorkspaceDeltaChange::ClipboardChanged(None)],
        };
        let response = SessionResponse {
            seq: first.seq,
            receipt: IntentReceipt::accepted().with_delta(delta),
            snapshot: None,
            selection: None,
        };
        let outcome = core.deliver(response, 0);
        assert!(outcome.applied);
        assert!(matches!(
            outcome.resync_reason,
            Some(ResyncReason::SequenceGap {
                expected: 3,
                got: 9
            })
        ));
        assert_eq!(core.mirror().projection_seq, 3, "mirror untouched on a gap");
    }

    /// A scripted executor that records what it was asked to run, for an
    /// end-to-end submit→execute→deliver loop without a real session.
    struct ScriptedExecutor;
    impl SessionExecutor for ScriptedExecutor {
        fn execute(&self, envelope: IntentEnvelope) -> SessionResponse {
            // Echo a snapshot stamped with the intent's seq — the in-process
            // transport would hand this straight to deliver().
            snapshot_response(envelope.seq, envelope.seq)
        }
    }

    #[test]
    fn in_process_loop_drives_a_real_executor_end_to_end() {
        let executor = ScriptedExecutor;
        let mut core = WorkerProxyCore::new(snapshot_at(0));
        // The in-process transport: submit, run, deliver — synchronously.
        let decision = core.submit(WorkspaceIntent::Recalculate);
        let envelope = decision.to_send.expect("first intent sent");
        let response = executor.execute(envelope);
        let outcome = core.deliver(response, 5);
        assert!(outcome.applied);
        assert_eq!(core.mirror().projection_seq, 1);
        assert!(core.pending().is_none(), "run cleared, nothing parked");
    }

    /// The real session, driven through the proxy + HostSessionExecutor exactly
    /// as the wasm worker will — just synchronous instead of postMessage. Proves
    /// the boundary carries the real engine's output (snapshot/delta + the
    /// MIXED-intent selection) end to end.
    #[test]
    fn real_session_flows_through_the_proxy_and_executor() {
        let _owner = Owner::new();
        let fixture = WorkspaceFixture::from_repo_fixture("accounts").expect("fixture loads");
        let model = WorkspaceModel::try_from(fixture).expect("model builds");
        let session = Arc::new(Mutex::new(
            TreeWorkspaceSession::from_model(&model).expect("session builds"),
        ));
        let executor = HostSessionExecutor::new(session);
        let mut core = WorkerProxyCore::new(executor.snapshot());
        let start_seq = core.mirror().projection_seq;
        let start_nodes = core.mirror().nodes_by_key.len();

        // AddNode is a MIXED intent: the engine adds + selects the new node, so
        // the response carries the new selection and a structural snapshot.
        let add = core.submit(WorkspaceIntent::AddNode {
            parent: None,
            symbol: "Zeta".to_string(),
            initial: InitialNodeContentProjection::Empty,
            is_meta: false,
        });
        let response = executor.execute(add.to_send.expect("add is sent"));
        let outcome = core.deliver(response, 0);
        assert!(outcome.applied);
        assert!(
            core.mirror().projection_seq > start_seq,
            "the real engine ran and the mirror advanced"
        );
        assert!(
            core.mirror().nodes_by_key.len() > start_nodes,
            "the new node is in the mirrored projection"
        );
        assert!(
            outcome.selection.is_some(),
            "AddNode is MIXED — the produced selection flows back for the proxy to mirror"
        );

        // Recalculate round-trips too and does not touch selection.
        let recalc = core.submit(WorkspaceIntent::Recalculate);
        let response = executor.execute(recalc.to_send.expect("recalc is sent"));
        let outcome = core.deliver(response, 0);
        assert!(outcome.applied);
        assert!(
            outcome.selection.is_none(),
            "Recalculate leaves the main-thread selection alone"
        );
    }
}

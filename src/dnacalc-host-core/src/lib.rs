//! DNA Calc host-core — the Leptos-free reference host (the Gap-4
//! SessionEngine crate).
//!
//! Host-core owns the document session and the model→intent seam between the
//! Skin IR wire protocol ([`dnacalc_skin_ir`]) and the OxCalc document surface
//! ([`oxcalc_core`]). It carries **no Leptos dependency anywhere in its tree**
//! (the no-Leptos gate, asserted by `cargo tree`), so a worker, a CLI/MCP host,
//! or the browser UI can each drive the same session logic without pulling in a
//! UI framework.
//!
//! ## Model-neutral sessions
//!
//! The common abstraction over document model families is a **closed enum, not
//! a trait** ([`DocumentSession`]): a general tree workspace
//! ([`DocumentSession::RichTree`]) or a strict-Excel workbook
//! ([`DocumentSession::Workbook`]). The two share almost no lifecycle beyond
//! "consume a `WorkspaceIntent`, publish a projection" — that pair is the common
//! surface for now; a trait is extracted only when a third family exists
//! (proof doc §Model-Neutral Sessions). Host-core matches per intent and returns
//! a typed [`IntentError::UnsupportedByModel`] receipt for an intent a family
//! does not support (e.g. `CreateScenario` on a workbook).
//!
//! ## H2 scope
//!
//! H2 stands up the crate seam: the [`DocumentSession`] enum, [`WorkbookSession`]
//! over one [`OxCalcDocumentContext`] (create workspace + add sheets + the
//! `set_grid_cell_value` write path), the [`HostCommand`] skeleton, the
//! [`ProjectionPublisher`] publication seam, and the Send/Sync audit below. The
//! universal `EnterGridCell` authored-entry verb, the tree-session migration
//! into host-core, xlsx, and the worker are all out of H2 scope.

pub mod command;
pub mod grid_publication;
pub mod workbook;

pub use command::{HostCommand, HostCommandOutcome, ProjectionPublisher, RecordingPublisher};
pub use grid_publication::grid_authored_cell_projection;
pub use workbook::{WorkbookSession, WorkbookSessionError};

use dnacalc_skin_ir::{IntentError, IntentReceipt, WorkspaceIntent};

// Re-export the engine document surface name the enum is built over, so callers
// name it through host-core rather than reaching into `oxcalc_core` directly.
pub use oxcalc_core::consumer::OxCalcDocumentContext;

/// The general-tree document model family — the seam placeholder for the
/// existing `TreeWorkspaceSession` (scenarios, sweeps, revision cursors,
/// `.dnatree` persistence), which lives in `dnatreecalc-host` today.
///
/// H2's NON-goals forbid a tree-session refactor "beyond the enum seam", and the
/// full `TreeWorkspaceSession` is reachable only through `dnatreecalc-host`,
/// which unconditionally links Leptos — pulling it into host-core would break
/// the no-Leptos gate. So the `RichTree` arm is a leptos-free **marker** in H2:
/// it establishes the closed-enum seam and gives the model-family dispatch a
/// second arm to distinguish, without moving any tree-session code. Migrating
/// the tree session into host-core is a later bead.
#[derive(Debug, Default)]
pub struct RichTreeSession {
    _seam: (),
}

impl RichTreeSession {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// A host document session: exactly one open document, of exactly one model
/// family. A closed enum (not a trait) per the model-neutral-sessions decision.
// `large_enum_variant`: the `RichTree` arm is a temporary 0-byte seam
// placeholder in H2 (the real tree session lives in `dnatreecalc-host`); once
// the tree session migrates into host-core the two variants balance. Boxing the
// workbook now would diverge from that end state and hand every caller an extra
// indirection for the enum's only live arm.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
#[non_exhaustive]
pub enum DocumentSession {
    /// The general-tree workspace model (seam placeholder in H2 — see
    /// [`RichTreeSession`]).
    RichTree(RichTreeSession),
    /// The strict-Excel workbook model, backed by one [`OxCalcDocumentContext`].
    Workbook(WorkbookSession),
}

impl DocumentSession {
    /// The model family's stable name, for diagnostics and the
    /// [`IntentError::UnsupportedByModel`] receipt.
    #[must_use]
    pub fn model_name(&self) -> &'static str {
        match self {
            DocumentSession::RichTree(_) => "RichTree",
            DocumentSession::Workbook(_) => "Workbook",
        }
    }

    /// Route a `WorkspaceIntent` to the session's model family. In H2 no
    /// grid-write intents are wired (the universal `EnterGridCell` verb is H6),
    /// so the workbook family supports no `WorkspaceIntent` yet and every intent
    /// — including a tree-only intent like `CreateScenario` — is answered with a
    /// typed [`IntentError::UnsupportedByModel`] receipt. The tree family is a
    /// seam placeholder and likewise routes nothing in H2.
    ///
    /// This is the per-intent model-family gate the proof doc specifies; the
    /// executable intent lanes attach in later beads.
    #[must_use]
    pub fn dispatch(&mut self, intent: WorkspaceIntent) -> IntentReceipt {
        IntentReceipt::rejected(IntentError::UnsupportedByModel {
            intent: workspace_intent_kind(&intent).to_string(),
            model: self.model_name().to_string(),
        })
    }
}

/// A stable, human-readable kind name for a `WorkspaceIntent`, used in the
/// [`IntentError::UnsupportedByModel`] receipt. Covers the families H2 must name
/// (notably the scenario family, per acceptance assertion 3) and falls back to a
/// generic label for the rest — the receipt's `model` field carries the
/// dispositive fact (which family rejected), so an exhaustive per-variant name
/// is not required in H2.
fn workspace_intent_kind(intent: &WorkspaceIntent) -> &'static str {
    match intent {
        WorkspaceIntent::CreateScenario { .. } => "CreateScenario",
        WorkspaceIntent::CreateScenarioFromCandidate { .. } => "CreateScenarioFromCandidate",
        WorkspaceIntent::ActivateScenario { .. } => "ActivateScenario",
        WorkspaceIntent::DeleteScenario { .. } => "DeleteScenario",
        WorkspaceIntent::SetScenarioOverride { .. } => "SetScenarioOverride",
        WorkspaceIntent::ClearScenarioOverride { .. } => "ClearScenarioOverride",
        WorkspaceIntent::CreateScenarioSweep { .. } => "CreateScenarioSweep",
        _ => "WorkspaceIntent",
    }
}

/// A [`HostCommand`] executor over a document session. H2 executes only the
/// `DispatchWorkspaceIntent` arm (the enum's sole H2 variant), routing through
/// [`DocumentSession::dispatch`].
impl DocumentSession {
    pub fn execute(&mut self, command: HostCommand) -> HostCommandOutcome {
        match command {
            HostCommand::DispatchWorkspaceIntent(intent) => {
                HostCommandOutcome::Dispatched(self.dispatch(intent))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Send/Sync audit (bead-required, W011 §"Send/Sync decision").
//
// The W011 decision hinged on whether `OxCalcDocumentContext` is `Send`.
//
// GROUND TRUTH (verified 2026-07-05 by attempting a `Send`/`Sync` static
// assertion — see the removed assertions in the bead's diff history):
// `OxCalcDocumentContext` is **NEITHER `Send` NOR `Sync`**. It transitively
// embeds `oxfunc_core::value::CalcValue`, whose `RichValue` payload is held
// behind a non-atomic `Rc<RichValue>`; the workspace-state map additionally
// holds a `NodeRef<Owned, ...>` handle that is itself `!Sync`. `WorkbookSession`
// and `DocumentSession` inherit `!Send + !Sync` from the context.
//
// W011 DISPOSITION (the `!Send` branch the proof doc pre-authored): host-core
// sessions are **single-threaded values**, not `Send` cross-thread handles.
// The existing `dnatreecalc-host` already reflects this — its `HOST_SESSIONS`
// registry is `thread_local` precisely because these engine types are `!Send`.
// So:
//   * Do NOT expose a `Dispatcher: Send + Sync` impl backed directly by a live
//     `DocumentSession`; a session stays on its owning thread (the wasm main
//     thread, or a single worker thread that owns its own context).
//   * A worker transport owns the context inside the worker thread and speaks
//     the serde wire protocol across `postMessage` — the context never crosses
//     a thread boundary as a value. This matches A.5's "engine placement is a
//     shell concern; front-end code binds only the `Dispatcher` trait + delta
//     mirror" and is the model-neutral seam H10 builds on.
//   * The `thread_local` registry is NOT re-invented in host-core; host-core
//     hands out plain owned `DocumentSession` values and lets the transport
//     decide affinity.
//
// The compile-time proof of `!Send` is the *absence* of a `Send` bound anywhere
// in this crate's public API — no `DocumentSession` field or return type claims
// `Send`, so a future edit that assumes it will fail to compile at the use site.
// The correct-direction static check we CAN enforce: the wire-protocol receipt
// type IS `Send + Sync` (it is pure serde data), which is what actually crosses
// the worker boundary.
// ---------------------------------------------------------------------------
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    // The receipt is pure serde data and DOES cross the thread boundary.
    assert_send_sync::<IntentReceipt>();
    assert_send_sync::<IntentError>();
    assert_send_sync::<HostCommand>();
    assert_send_sync::<HostCommandOutcome>();
};

#[cfg(test)]
mod tests {
    use super::*;
    use oxfunc_core::value::CalcValue;

    /// Acceptance (2): create a workbook session → `sheets()` projects one
    /// sheet → `set_grid_cell_value(A1, 7)` → snapshot shows `7`.
    #[test]
    fn workbook_session_add_sheet_write_a1_reads_back_seven() {
        let mut session = WorkbookSession::create("workbook:h2-accept").unwrap();

        // A freshly-created workbook has no sheets yet; adding one projects
        // exactly one sheet through `sheets()`.
        assert!(session.sheets().unwrap().is_empty());
        let sheet = session.add_sheet("Sheet1").unwrap();
        let rows = session.sheets().unwrap();
        assert_eq!(rows.len(), 1, "exactly one sheet after add_sheet");
        assert_eq!(rows[0].display_name, "Sheet1");
        assert_eq!(rows[0].node_id, sheet);
        assert!(rows[0].grid_backed, "added sheet is grid-backed");

        // Write A1 = 7 via the H2 write path (`set_grid_cell_value`, row 1 /
        // col 1 = A1), then read the published value back from the snapshot.
        session
            .set_grid_cell_value(sheet, 1, 1, CalcValue::number(7.0))
            .unwrap();
        let a1 = session.grid_cell_value(sheet, 1, 1).unwrap();
        assert_eq!(
            a1.and_then(|value| value.as_number()),
            Some(7.0),
            "snapshot shows 7 at A1"
        );
    }

    /// Acceptance (3): `IntentError::UnsupportedByModel` receipt for
    /// `CreateScenario` on a Workbook session.
    #[test]
    fn create_scenario_on_workbook_is_unsupported_by_model() {
        let session = WorkbookSession::create("workbook:h2-unsupported").unwrap();
        let mut document = DocumentSession::Workbook(session);

        let receipt = document.dispatch(WorkspaceIntent::CreateScenario {
            scenario_id: "s1".to_string(),
            name: "Downside".to_string(),
            base_scenario_id: None,
        });

        assert!(!receipt.accepted, "workbook rejects CreateScenario");
        match receipt.error {
            Some(IntentError::UnsupportedByModel { intent, model }) => {
                assert_eq!(intent, "CreateScenario");
                assert_eq!(model, "Workbook");
            }
            other => panic!("expected UnsupportedByModel receipt, got {other:?}"),
        }
    }

    /// The publication seam records receipts a host publishes through it.
    #[test]
    fn recording_publisher_captures_published_receipts() {
        let publisher = RecordingPublisher::new();
        let mut document =
            DocumentSession::Workbook(WorkbookSession::create("workbook:h2-publish").unwrap());

        let outcome = document.execute(HostCommand::DispatchWorkspaceIntent(
            WorkspaceIntent::CreateScenario {
                scenario_id: "s1".to_string(),
                name: "Downside".to_string(),
                base_scenario_id: None,
            },
        ));
        let HostCommandOutcome::Dispatched(receipt) = outcome;
        publisher.publish(&receipt);

        let published = publisher.published();
        assert_eq!(published.len(), 1);
        assert!(!published[0].accepted);
    }
}

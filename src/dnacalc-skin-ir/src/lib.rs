//! DNA Calc skin IR — the pure UX wire protocol.
//!
//! This crate is the stable Skin IR: intents, receipts, deltas, projections,
//! identity, selection, grid interest/projection, the session-channel
//! protocol, and the pure `SkinState` / `SharedSkinState` contracts. It
//! carries **no Leptos dependency anywhere in its tree** (dev-dependencies
//! included) so that any wire-protocol consumer — worker, CLI/MCP, browser
//! UI, or a future multi-skin host — can depend on the protocol without
//! pulling in a UI framework.
//!
//! The Leptos-bound mounting layer (skin trait, registry, signal-backed
//! state handles, view adapters) lives in `dnacalc-skin-leptos`, which
//! re-exports this crate.
//!
//! ## Version skew policy
//!
//! Protocol messages are `serde`-encoded. Unknown intent/delta variants are
//! **rejected** at decode time — a version-skewed peer gets a typed decode
//! failure, never a silently-ignored intent. Handshakes carry
//! [`SKIN_IR_PROTOCOL_SCHEMA`] so peers can detect skew before exchanging
//! messages.

pub mod command;
pub mod dispatcher;
pub mod identity;
pub mod intent;
pub mod keychord;
pub mod manifest;
pub mod permissions;
pub mod preview;
pub mod selection;
pub mod session_channel;
pub mod state;
pub mod workspace;

/// Wire-protocol schema identifier carried on session handshakes (worker
/// `Init`/`Ready`, future MCP hello). Peers compare this before exchanging
/// intents so version skew is detected up front.
pub const SKIN_IR_PROTOCOL_SCHEMA: &str = "dnacalc.skin_ir.v1";

pub use command::{CommandCatalogProjection, CommandIntentKindProjection, CommandMetaProjection};
pub use dispatcher::RecordingDispatcher;
pub use identity::{NodeId, NodeKey, SkinId, SkinMountSlot};
pub use intent::{
    AuthoringScope, ClipboardPayloadKind, DependencyDeltaProjection, Dispatcher,
    FormulaReferenceInsertionProjection, FormulaReferenceInsertionTarget, IntentError,
    IntentReceipt, IntentRecord, NodeAttributePatch, NodeValueDeltaProjection, ReplayOutcome,
    StructuralDeltaProjection, SweepPointInput, TableCellInput, TableRowInput, WorkspaceDelta,
    WorkspaceDeltaChange, WorkspaceIntent, replay,
};
pub use keychord::{KeyChord, KeybindingError, KeybindingOverrideMap, SkinVerb};
pub use manifest::{CapabilityError, SkinCapabilities, SkinCategory, SkinManifest};
pub use permissions::Persona;
pub use preview::{PreviewError, PreviewService};
pub use selection::{SelectionState, TableCellSelection};
pub use session_channel::{
    FrameMetric, IntentEnvelope, PendingIntentQueue, PendingRun, ResyncReason, SessionResponse,
    apply_delta, change_kind, delta_is_fully_applicable, is_delta_applicable,
};
pub use state::{
    InMemorySkinStatePersistenceStore, MigrationError, PersistedSkinStateRecord,
    SHARED_AUDIT_CAPACITY, SharedSkinState, SharedStateAuditRecord, SharedStateChange,
    SharedStateOrigin, SkinState, SkinStatePersistenceError, SkinStatePersistenceKey,
    SkinStatePersistenceStore, WorkspaceRecalcMode,
};

#[cfg(not(target_arch = "wasm32"))]
pub use state::LocalFileSkinStatePersistenceStore;
pub use workspace::{
    ActiveNodeDetailProjection, ActiveSelectionDetailProjection, ActiveTableCellDetailProjection,
    AuthoringScopeExpansionError, BindingDiagnosticProjection, CalcRunProjection,
    CalcRunStateProjection, CandidateNodeProjection, CandidateProjection, CleaveFilter,
    CleavePredicate, CleaveSort, ClipboardNodeFormatProjection, ClipboardNodeValueProjection,
    ClipboardOperationProjection, ClipboardPayloadProjection, ClipboardProjection,
    ComparativeColumnProjection, ComparativeProjection, ComparativeSourceProjection,
    DependencyDescriptorProjection, DependencyEdgeProjection, DependencyGraphProjection,
    DependencyKindProjection, DerivationHoleBindingProjection, DerivationInvocationProjection,
    DerivationOxfmlTraceEventProjection, DerivationPreparedArgumentProjection,
    DerivationTemplateHoleProjection, DerivationTemplateSelectionProjection,
    DerivationTraceProjection, EffectiveFormatProjection, FormatSourceProjection,
    FormulaBindPreviewDiagnosticProjection, FormulaBindPreviewDiagnosticStage,
    FormulaBindPreviewInputKind, FormulaBindPreviewProfileViolationKindProjection,
    FormulaBindPreviewProfileViolationProjection, FormulaBindPreviewProjection, GridCellProjection,
    GridMergedOverlayDescriptor, GridOverlayBundle, GridOverlayRect, GridProjection,
    GridSpillOverlayDescriptor, GridTableColumnBand, GridTableOverlayDescriptor,
    InitialNodeContentProjection, InvalidationReasonProjection,
    MutationImpactBlockedReasonProjection, MutationImpactIntentProjection,
    MutationImpactProjection, NameCollisionProjection, NodeCalcStateProjection, NodeClassification,
    NodeContentKind, NodeInvalidationProjection, NodeNoteProjection, NodeValueProjection, NodeView,
    NoteSourceProjection, PhaseKeyProjection, RecalcPlanInvalidationProjection, RecalcPlanMutation,
    RecalcPlanProjection, ReferenceResolutionProjection, ReferenceTargetProjection,
    RevisionHistoryEntryProjection, RevisionHistoryProjection,
    RevisionInvalidationSummaryEntryProjection, RevisionTransactionSummaryProjection,
    RuntimeEffectFamilyProjection, RuntimeEffectProjection, RuntimeOverlayKindProjection,
    RuntimeOverlayProjection, ScenarioManifestProjection, ScenarioProjection,
    ScenarioSourceProjection, SeriesManifestProjection, SeriesPointProjection, SeriesProjection,
    SourceSpanProjection, SpeculationPressureProjection, SweepManifestProjection,
    SweepPointProjection, SweepProjection, TableAnchorProjection, TableCellEditabilityProjection,
    TableCellProjection, TableCellRegionProjection, TableCellsProjection,
    TableColumnBodyProjection, TableColumnProjection, TableDependencyFactBlockerProjection,
    TableDependencyFactKindProjection, TableDependencyFactProjection,
    TableDependencyFactStatusProjection, TableFormulaBindPreviewProjection,
    TableFormulaMetadataProjection, TableProjection, TableRowProjection,
    TemplateManifestProjection, TemplateProjection, TreeReferenceCollectionFamilyProjection,
    TreeReferenceCollectionProjection, WorkspaceRevisionProjection, WorkspaceState,
};

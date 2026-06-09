//! DNA TreeCalc skin framework.
//!
//! Host-agnostic contract layer for the Winamp-style skin architecture
//! (`docs/ux/SKINS.md`). Defines what a skin gets (`SkinContext`),
//! what it provides (`WorkspaceSkin`, `SkinHandle`), what it persists
//! (`SkinState`), and how the registry orchestrates mounting and
//! switching. No DnaTreeCalc-specific knowledge — implementations of
//! the trait live in `dnatreecalc-skins`; the shell that composes
//! mounted skins lives in `dnatreecalc-shell`.

pub mod accessibility;
pub mod command;
pub mod identity;
pub mod intent;
pub mod manifest;
pub mod registry;
pub mod selection;
pub mod skin;
pub mod state;
pub mod theme;
pub mod workspace;

pub use accessibility::{
    AriaAttrs, SelectableItemA11y, SelectableRowA11y, aria_bool, listbox_a11y, roving_tabindex,
    stable_node_dom_id, table_a11y, tree_a11y,
};
pub use command::{CommandCatalogProjection, CommandIntentKindProjection, CommandMetaProjection};
pub use identity::{NodeId, NodeKey, SkinId, SkinMountSlot};
pub use intent::{
    AuthoringScope, ClipboardPayloadKind, DependencyDeltaProjection, Dispatcher,
    FormulaReferenceInsertionProjection, FormulaReferenceInsertionTarget, InMemoryDispatcher,
    IntentError, IntentReceipt, NodeAttributePatch, NodeValueDeltaProjection,
    StructuralDeltaProjection, SweepPointInput, TableCellInput, TableRowInput, WorkspaceDelta,
    WorkspaceDeltaChange, WorkspaceIntent,
};
pub use manifest::{SkinCapabilities, SkinCategory, SkinManifest};
pub use registry::SkinRegistry;
pub use selection::{SelectionState, TableCellSelection};
pub use skin::{
    ErasedSkinContext, ErasedSkinFactory, RegisteredSkin, SkinContext, SkinHandle, WorkspaceSkin,
};
pub use state::{
    InMemorySkinStatePersistenceStore, MigrationError, PersistedSkinStateRecord, SharedSkinState,
    SharedSkinStateHandle, SkinState, SkinStateHandle, SkinStatePersistenceError,
    SkinStatePersistenceKey, SkinStatePersistenceStore, WorkspaceRecalcMode,
};
pub use theme::{ThemeMode, ThemeTokens};

#[cfg(not(target_arch = "wasm32"))]
pub use state::LocalFileSkinStatePersistenceStore;
pub use workspace::{
    ActiveNodeDetailProjection, ActiveSelectionDetailProjection, ActiveTableCellDetailProjection,
    AuthoringScopeExpansionError, BindingDiagnosticProjection, CalcRunProjection,
    CalcRunStateProjection, CandidateNodeProjection, CandidateProjection,
    ClipboardNodeFormatProjection, ClipboardNodeValueProjection, ClipboardOperationProjection,
    ClipboardPayloadProjection, ClipboardProjection, ComparativeColumnProjection,
    ComparativeProjection, ComparativeSourceProjection, DependencyDescriptorProjection,
    DependencyEdgeProjection, DependencyGraphProjection, DependencyKindProjection,
    DerivationHoleBindingProjection, DerivationInvocationProjection,
    DerivationOxfmlTraceEventProjection, DerivationPreparedArgumentProjection,
    DerivationTemplateHoleProjection, DerivationTemplateSelectionProjection,
    DerivationTraceProjection, EffectiveFormatProjection, FormatSourceProjection,
    FormulaBindPreviewDiagnosticProjection, FormulaBindPreviewDiagnosticStage,
    FormulaBindPreviewInputKind, FormulaBindPreviewProfileViolationKindProjection,
    FormulaBindPreviewProfileViolationProjection, FormulaBindPreviewProjection,
    InitialNodeContentProjection, InvalidationReasonProjection,
    MutationImpactBlockedReasonProjection, MutationImpactIntentProjection,
    MutationImpactProjection, NameCollisionProjection, NodeCalcStateProjection, NodeContentKind,
    NodeInvalidationProjection, NodeNoteProjection, NodeValueProjection, NodeView,
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

#[cfg(test)]
mod tests;

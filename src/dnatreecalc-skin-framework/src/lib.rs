//! DNA TreeCalc skin framework.
//!
//! Host-agnostic contract layer for the Winamp-style skin architecture
//! (`docs/ux/SKINS.md`). Defines what a skin gets (`SkinContext`),
//! what it provides (`WorkspaceSkin`, `SkinHandle`), what it persists
//! (`SkinState`), and how the registry orchestrates mounting and
//! switching. No DnaTreeCalc-specific knowledge — implementations of
//! the trait live in `dnatreecalc-skins`; the shell that composes
//! mounted skins lives in `dnatreecalc-shell`.

pub mod identity;
pub mod intent;
pub mod manifest;
pub mod registry;
pub mod selection;
pub mod skin;
pub mod state;
pub mod workspace;

pub use identity::{NodeId, NodeKey, SkinId, SkinMountSlot};
pub use intent::{
    DependencyDeltaProjection, Dispatcher, InMemoryDispatcher, IntentError, IntentReceipt,
    NodeValueDeltaProjection, StructuralDeltaProjection, TableCellInput, TableRowInput,
    WorkspaceDelta, WorkspaceDeltaChange, WorkspaceIntent,
};
pub use manifest::{SkinCapabilities, SkinCategory, SkinManifest};
pub use registry::SkinRegistry;
pub use selection::{SelectionState, TableCellSelection};
pub use skin::{
    ErasedSkinContext, ErasedSkinFactory, RegisteredSkin, SkinContext, SkinHandle, WorkspaceSkin,
};
pub use state::{
    MigrationError, SharedSkinState, SharedSkinStateHandle, SkinState, SkinStateHandle,
    WorkspaceRecalcMode,
};
pub use workspace::{
    ActiveNodeDetailProjection, ActiveSelectionDetailProjection, ActiveTableCellDetailProjection,
    BindingDiagnosticProjection, CalcRunProjection, CalcRunStateProjection,
    DependencyDescriptorProjection, DependencyEdgeProjection, DependencyGraphProjection,
    DependencyKindProjection, DerivationHoleBindingProjection, DerivationInvocationProjection,
    DerivationOxfmlTraceEventProjection, DerivationPreparedArgumentProjection,
    DerivationTemplateHoleProjection, DerivationTemplateSelectionProjection,
    DerivationTraceProjection, InvalidationReasonProjection, NodeCalcStateProjection,
    NodeContentKind, NodeInvalidationProjection, NodeValueProjection, NodeView, PhaseKeyProjection,
    ReferenceResolutionProjection, ReferenceTargetProjection, RuntimeEffectFamilyProjection,
    RuntimeEffectProjection, RuntimeOverlayKindProjection, RuntimeOverlayProjection,
    SourceSpanProjection, TableAnchorProjection, TableCellEditabilityProjection,
    TableCellProjection, TableCellRegionProjection, TableCellsProjection,
    TableColumnBodyProjection, TableColumnProjection, TableDependencyFactBlockerProjection,
    TableDependencyFactKindProjection, TableDependencyFactProjection,
    TableDependencyFactStatusProjection, TableFormulaMetadataProjection, TableProjection,
    TableRowProjection, TreeReferenceCollectionFamilyProjection, TreeReferenceCollectionProjection,
    WorkspaceRevisionProjection, WorkspaceState,
};

#[cfg(test)]
mod tests;

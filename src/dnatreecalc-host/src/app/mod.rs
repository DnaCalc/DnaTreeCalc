//! Host-side wiring between UI/session projection and the direct OxCalc
//! context. Nothing here knows about Leptos render details; the shell and
//! skin crates own that.

mod dispatcher;
mod persistence;
mod preview;
mod projection;
mod registry;
mod session;
mod templates;
mod workbook_dispatcher;
mod worker_proxy;

pub use dispatcher::HostDispatcher;
#[cfg(not(target_arch = "wasm32"))]
pub use persistence::LocalFileWorkspaceDocumentStore;
pub use persistence::{
    InMemoryWorkspaceDocumentStore, WorkspaceDocumentCatalog, WorkspaceDocumentStore,
    WorkspaceDocumentStoreError, workspace_session_from_document_store_or_default,
};
pub use preview::{preview_accounts_workspace_session, preview_accounts_workspace_state};
pub use projection::workspace_state_from_model;
pub use registry::build_default_registry;
pub use session::{
    DnaTreeWorkspaceDocument, TreeWorkspaceCollectionDependencyProjection, TreeWorkspaceSession,
    TreeWorkspaceSessionError,
};
pub use workbook_dispatcher::WorkbookHostDispatcher;
pub use worker_proxy::{
    DeliverOutcome, HostSessionExecutor, SessionExecutor, SubmitDecision, WorkerProxyCore,
};

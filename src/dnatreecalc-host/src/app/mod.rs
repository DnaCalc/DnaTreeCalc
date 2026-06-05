//! Host-side wiring between UI/session projection and the direct OxCalc
//! context. Nothing here knows about Leptos render details; the shell and
//! skin crates own that.

mod dispatcher;
mod preview;
mod projection;
mod registry;
mod session;

pub use dispatcher::HostDispatcher;
pub use preview::{preview_accounts_workspace_session, preview_accounts_workspace_state};
pub use projection::workspace_state_from_model;
pub use registry::build_default_registry;
pub use session::{
    TreeWorkspaceCollectionDependencyProjection, TreeWorkspaceSession, TreeWorkspaceSessionError,
};

//! Host-side wiring between the workspace model + OxCalc bridge and the
//! skin framework / shell. Nothing here knows about Leptos render details;
//! the shell and skin crates own that.

mod dispatcher;
mod preview;
mod projection;
mod registry;

pub use dispatcher::HostDispatcher;
pub use preview::preview_accounts_workspace_state;
pub use projection::workspace_state_from_model;
pub use registry::build_default_registry;

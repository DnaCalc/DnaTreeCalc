use dnatreecalc_skin_framework::WorkspaceState;

use crate::model::{WorkspaceFixture, WorkspaceModel};

use super::session::TreeWorkspaceSession;

/// JSON for the canonical `accounts` corpus workspace, embedded at compile
/// time so the WASM build does not need filesystem access. Native code
/// paths can still load the same fixture from disk through
/// `WorkspaceFixture::from_repo_fixture` for tests that vary it.
const ACCOUNTS_FIXTURE_JSON: &str =
    include_str!("../../../../docs/test-corpus/workspaces/accounts.json");

/// Build a ready-to-mount `WorkspaceState` from the embedded accounts
/// fixture. Used by the web entry to give the walking-skeleton shell a
/// real workspace to render against.
///
/// # Panics
///
/// Only if the embedded fixture stops parsing as a valid
/// `WorkspaceFixture` — caught immediately by `cargo test`.
#[must_use]
pub fn preview_accounts_workspace_state() -> WorkspaceState {
    let session = preview_accounts_workspace_session();
    session
        .workspace_state()
        .expect("accounts fixture must project from OxCalc")
}

#[must_use]
pub fn preview_accounts_workspace_session() -> TreeWorkspaceSession {
    let fixture: WorkspaceFixture = serde_json::from_str(ACCOUNTS_FIXTURE_JSON)
        .expect("embedded accounts.json must parse as a WorkspaceFixture");
    let model =
        WorkspaceModel::try_from(fixture).expect("accounts fixture must form a valid model");
    let mut session = TreeWorkspaceSession::from_model(&model)
        .expect("accounts fixture must build OxCalc context");
    session
        .recalculate()
        .expect("accounts fixture must recalculate through OxCalc");
    session
}

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::NodeId;

    #[test]
    fn embedded_accounts_fixture_parses() {
        let state = preview_accounts_workspace_state();
        assert_eq!(state.workspace_id, "accounts");
        assert!(
            state
                .node(&NodeId::new("Accounts.2005.Q1.Income"))
                .is_some()
        );
    }
}

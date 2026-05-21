#![forbid(unsafe_code)]

//! DNA TreeCalc host crate.
//!
//! W002 starts with the non-UI host model and OxCalc bridge boundary. UI
//! shell, skins, and persistence land in later worksets.

pub mod adapters;
pub mod model;

#[cfg(test)]
mod tests {
    use crate::model::{NodeContentKind, WorkspaceFixture, WorkspaceModel};

    #[test]
    fn loads_accounts_fixture_as_tree_workspace() {
        let fixture = WorkspaceFixture::from_repo_fixture("accounts").unwrap();
        let workspace = WorkspaceModel::try_from(fixture).unwrap();

        let root = workspace.node("Accounts").unwrap();
        assert_eq!(root.child_paths, vec!["Accounts.2005"]);

        let q1 = workspace.node("Accounts.2005.Q1").unwrap();
        assert_eq!(
            q1.child_paths,
            vec![
                "Accounts.2005.Q1.Income",
                "Accounts.2005.Q1.Margin",
                "Accounts.2005.Q1.Net"
            ]
        );

        let margin = workspace.node("Accounts.2005.Q1.Margin").unwrap();
        assert_eq!(margin.content.kind(), NodeContentKind::Constant);
        assert_eq!(margin.content.text(), "0.2");

        let income = workspace.node("Accounts.2005.Q1.Income").unwrap();
        assert_eq!(income.content.kind(), NodeContentKind::Formula);
        assert_eq!(income.content.text(), "=Sales*Margin");
    }
}
